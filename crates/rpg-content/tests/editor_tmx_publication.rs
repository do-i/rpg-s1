//! Check actual Python editor publications with the same parser used by the engine.
use std::{path::Path, process::Command};

use rpg_content::{
    scenario_path::ScenarioRelativePath,
    tmx_header::{TmxPropertyValue, parse_tmx_map_document},
};

#[test]
fn python_editor_create_retarget_delete_remain_rust_readable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let python = std::env::var_os("RPG_S1_EDITOR_TEST_PYTHON").unwrap_or_else(|| "python3".into());
    let output = Command::new(python)
        .current_dir(root)
        .args([
            "-c",
            r#"
from pathlib import Path
from tempfile import TemporaryDirectory
from tools.map_editor.edit.tmx_writer import create_portal, save_portal_target, delete_portal

with TemporaryDirectory(prefix="rpg-editor-rust-test-") as directory:
    path = Path(directory) / "source.tmx"
    path.write_bytes(Path("tools/map_editor/tests/fixtures/source.tmx").read_bytes())
    portal = create_portal(path, (64, 32, 32, 32), "target", (2, 1))
    print(path.read_text(), end="\0")
    save_portal_target(path, portal, "source", (1, 2), (32, 64, 32, 32))
    print(path.read_text(), end="\0")
    delete_portal(path, portal)
    print(path.read_text(), end="\0")
"#,
        ])
        .output()
        .expect("python3 (or RPG_S1_EDITOR_TEST_PYTHON) must run the stdlib-only writer");
    assert!(
        output.status.success(),
        "Python writer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let xml = String::from_utf8(output.stdout).unwrap();
    let logical = ScenarioRelativePath::try_from("media/maps/source.tmx").unwrap();
    let documents = xml
        .split_terminator('\0')
        .map(|xml| parse_tmx_map_document(xml, &logical).expect("published TMX must parse"))
        .collect::<Vec<_>>();
    assert_eq!(documents.len(), 3);
    for (index, document) in documents.iter().enumerate() {
        assert_eq!(document.header().width(), 4);
        assert_eq!(document.tile_layers()[0].gids().len(), 16);
        let objects = document.object_groups()[0].objects();
        assert_eq!(objects.len(), if index == 2 { 1 } else { 2 });
        assert_eq!(objects[0].id(), 1);
        if index == 2 {
            continue;
        }
        let portal = &objects[1];
        assert_eq!(portal.id(), 2);
        let (map, x, y, source_x, source_y) = if index == 0 {
            ("target", 2, 1, 64.0, 32.0)
        } else {
            ("source", 1, 2, 32.0, 64.0)
        };
        assert_eq!(portal.x(), source_x);
        assert_eq!(portal.y(), source_y);
        let properties = portal.properties();
        assert_eq!(properties[0].name(), "target_map");
        assert_eq!(properties[0].value(), &TmxPropertyValue::String(map.into()));
        assert_eq!(properties[1].name(), "target_position_x");
        assert_eq!(properties[1].value(), &TmxPropertyValue::Integer(x));
        assert_eq!(properties[2].name(), "target_position_y");
        assert_eq!(properties[2].value(), &TmxPropertyValue::Integer(y));
    }
}
