//! Exhaustive structural traversal for scenario dialogue graphs.

use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    scenario_dialogue::{DialogueDocument, DialogueEntry, EntryDialogue},
    scenario_manifest::Manifest,
    scenario_yaml,
    world_dialogue::validate_graph,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DialogueTraversalReport {
    pub(crate) scenario_id: Option<String>,
    pub(crate) scenario_name: Option<String>,
    pub(crate) documents: Vec<DialogueTraversalDocument>,
    pub(crate) load_error: Option<String>,
}

impl DialogueTraversalReport {
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            load_error: Some(message.into()),
            ..Self::default()
        }
    }

    pub(crate) fn root_branches(&self) -> usize {
        self.documents.iter().map(|document| document.roots).sum()
    }

    pub(crate) fn terminating_paths(&self) -> usize {
        self.documents
            .iter()
            .map(|document| document.terminating_paths)
            .sum()
    }

    pub(crate) fn cycle_count(&self) -> usize {
        self.documents
            .iter()
            .map(|document| document.cycles.len())
            .sum()
    }

    pub(crate) fn error_count(&self) -> usize {
        self.documents
            .iter()
            .map(|document| document.errors.len())
            .sum::<usize>()
            + usize::from(self.load_error.is_some())
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.error_count() == 0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DialogueTraversalDocument {
    pub(crate) id: String,
    pub(crate) roots: usize,
    pub(crate) terminating_paths: usize,
    pub(crate) cycles: Vec<String>,
    pub(crate) errors: Vec<String>,
}

pub(crate) fn build_dialogue_traversal_sweep(physical_root: &Path) -> DialogueTraversalReport {
    let manifest_text = match fs::read_to_string(physical_root.join("manifest.yaml")) {
        Ok(text) => text,
        Err(error) => {
            return DialogueTraversalReport::with_load_error(format!(
                "manifest.yaml could not be read: {error}"
            ));
        }
    };
    let manifest: Manifest = match scenario_yaml::from_str(&manifest_text) {
        Ok(manifest) => manifest,
        Err(error) => {
            return DialogueTraversalReport::with_load_error(format!(
                "manifest.yaml is invalid: {error}"
            ));
        }
    };
    let directory = physical_root.join(manifest.refs.dialogue.as_str());
    let paths = match yaml_paths(&directory) {
        Ok(paths) => paths,
        Err(error) => return DialogueTraversalReport::with_load_error(error),
    };
    let documents = paths.into_iter().map(|path| analyze_path(&path)).collect();

    DialogueTraversalReport {
        scenario_id: Some(manifest.id),
        scenario_name: Some(manifest.name),
        documents,
        load_error: None,
    }
}

fn yaml_paths(directory: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("dialogue directory could not be read: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("dialogue directory entry could not be read: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn analyze_path(path: &Path) -> DialogueTraversalDocument {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("non_utf8_dialogue")
        .to_owned();
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            return DialogueTraversalDocument {
                id: stem,
                errors: vec![format!("document could not be read: {error}")],
                ..DialogueTraversalDocument::default()
            };
        }
    };
    let document: DialogueDocument = match scenario_yaml::from_str(&text) {
        Ok(document) => document,
        Err(error) => {
            return DialogueTraversalDocument {
                id: stem,
                errors: vec![format!("document is invalid: {error}")],
                ..DialogueTraversalDocument::default()
            };
        }
    };
    let id = document.effective_id(&stem).to_owned();
    match document {
        DialogueDocument::Entries(dialogue) => analyze_entries(id, &dialogue),
        DialogueDocument::Cutscene(_) | DialogueDocument::LinePool(_) => {
            DialogueTraversalDocument {
                id,
                roots: 1,
                terminating_paths: 1,
                ..DialogueTraversalDocument::default()
            }
        }
    }
}

fn analyze_entries(id: String, dialogue: &EntryDialogue) -> DialogueTraversalDocument {
    if let Err(error) = validate_graph(dialogue) {
        return DialogueTraversalDocument {
            id,
            errors: vec![error.to_string()],
            ..DialogueTraversalDocument::default()
        };
    }
    let roots = dialogue
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.node.is_none())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return DialogueTraversalDocument {
            id,
            errors: vec!["dialogue has no condition-selected root entry".to_owned()],
            ..DialogueTraversalDocument::default()
        };
    }
    let nodes = dialogue
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.node.as_ref().map(|node| (node.as_str(), index)))
        .collect::<BTreeMap<_, _>>();
    let mut terminating_paths = 0;
    let mut cycles = Vec::new();
    for &root in &roots {
        walk(
            root,
            &dialogue.entries,
            &nodes,
            &mut Vec::new(),
            &mut terminating_paths,
            &mut cycles,
        );
    }
    cycles.sort();
    cycles.dedup();
    DialogueTraversalDocument {
        id,
        roots: roots.len(),
        terminating_paths,
        cycles,
        errors: Vec::new(),
    }
}

fn walk(
    index: usize,
    entries: &[DialogueEntry],
    nodes: &BTreeMap<&str, usize>,
    stack: &mut Vec<usize>,
    terminating_paths: &mut usize,
    cycles: &mut Vec<String>,
) {
    if let Some(start) = stack.iter().position(|candidate| *candidate == index) {
        let mut cycle = stack[start..]
            .iter()
            .map(|&entry| entry_label(entry, &entries[entry]))
            .collect::<Vec<_>>();
        cycle.push(entry_label(index, &entries[index]));
        cycles.push(cycle.join(" -> "));
        return;
    }
    let entry = &entries[index];
    if entry.end {
        *terminating_paths += 1;
        return;
    }
    let targets = if entry.choices.is_empty() {
        entry.next.iter().map(String::as_str).collect::<Vec<_>>()
    } else {
        entry
            .choices
            .iter()
            .map(|choice| choice.target.as_str())
            .collect::<Vec<_>>()
    };
    if targets.is_empty() {
        *terminating_paths += 1;
        return;
    }
    stack.push(index);
    for target in targets {
        walk(
            *nodes
                .get(target)
                .expect("production graph validation checked every target"),
            entries,
            nodes,
            stack,
            terminating_paths,
            cycles,
        );
    }
    stack.pop();
}

fn entry_label(index: usize, entry: &DialogueEntry) -> String {
    entry
        .node
        .as_ref()
        .map(|node| format!("node:{node}"))
        .unwrap_or_else(|| format!("root[{index}]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(yaml: &str) -> EntryDialogue {
        let DialogueDocument::Entries(dialogue) = scenario_yaml::from_str(yaml).unwrap() else {
            panic!("expected entries dialogue");
        };
        dialogue
    }

    #[test]
    fn traverses_every_choice_to_a_terminal_node() {
        let dialogue = entries(
            "entries:\n  - lines: [Start]\n    choices:\n      - {text: Left, target: left}\n      - {text: Right, target: right}\n  - {node: left, lines: [Done], end: true}\n  - {node: right, lines: [Done]}\n",
        );

        let report = analyze_entries("branching".to_owned(), &dialogue);

        assert_eq!(report.roots, 1);
        assert_eq!(report.terminating_paths, 2);
        assert!(report.cycles.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn reports_cycle_with_the_exact_node_path() {
        let dialogue = entries(
            "entries:\n  - {lines: [Start], next: a}\n  - {node: a, lines: [A], next: b}\n  - {node: b, lines: [B], next: a}\n",
        );

        let report = analyze_entries("cyclic".to_owned(), &dialogue);

        assert_eq!(report.terminating_paths, 0);
        assert_eq!(report.cycles, ["node:a -> node:b -> node:a"]);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn uses_production_graph_validation_for_missing_targets() {
        let dialogue = entries("entries:\n  - {lines: [Start], next: missing}\n");

        let report = analyze_entries("broken".to_owned(), &dialogue);

        assert_eq!(report.errors, ["dialogue targets missing node `missing`"]);
    }
}
