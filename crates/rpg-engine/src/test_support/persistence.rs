//! Disposable native saves shared by persistence and replay regressions.

use std::{fs, path::PathBuf, process::Command, time::Duration};

use crate::{
    game_state::GameState,
    new_game::{NewGameScenario, build_new_game_state},
    save_data::{NativeSaveEnvelope, tests::fixture_balance},
    save_store::SaveStore,
    scenario_class::ClassDefinition,
    scenario_manifest::Manifest,
    scenario_party::PartyCatalog,
    scenario_yaml,
};

pub(crate) struct FixtureDirectory(pub PathBuf);

impl FixtureDirectory {
    pub(crate) fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../temp/refactoring");
        fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!(
            "rf10-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    pub(crate) fn child(&self, test: &str, phase: &str) {
        // Use the current libtest executable, not a second App in this process. Output goes
        // to files so a verbose child cannot block on a full pipe. Bound a hung child too.
        let output = fs::File::create(self.0.join(format!("{phase}.log"))).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env("RPG_S1_REGRESSION_CHILD", phase)
            .env("RPG_S1_REGRESSION_ROOT", &self.0)
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                let log = fs::read_to_string(self.0.join(format!("{phase}.log"))).unwrap();
                assert!(status.success(), "child {phase} failed: {log}");
                assert!(log.contains("1 passed"), "child test did not run: {log}");
                break;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("child {phase} exceeded 30 seconds");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

pub(crate) fn fresh_game() -> GameState {
    let manifest: Manifest =
        scenario_yaml::from_str(include_str!(scenario_file!("manifest.yaml"))).unwrap();
    let party: PartyCatalog =
        scenario_yaml::from_str(include_str!(scenario_file!("data/party.yaml"))).unwrap();
    let class: ClassDefinition =
        scenario_yaml::from_str(include_str!(scenario_file!("data/classes/hero.yaml"))).unwrap();
    build_new_game_state(
        NewGameScenario {
            manifest: &manifest,
            party: &party,
            balance: &fixture_balance(),
            protagonist_class: &class,
        },
        Duration::ZERO,
    )
    .unwrap()
}

pub(crate) fn save(store: &SaveStore, slot: usize, game: &GameState) {
    let envelope = NativeSaveEnvelope::from_game_state(
        game,
        "my_rpg_story",
        "1.0.0",
        1,
        game.map().current().unwrap().as_str(),
    )
    .unwrap();
    store
        .write(slot, &envelope, false, &fixture_balance())
        .unwrap();
}

pub(crate) fn load(store: &SaveStore, slot: usize) -> GameState {
    let (envelope, game) = store
        .load(slot, "my_rpg_story", "1.0.0", &fixture_balance())
        .unwrap();
    assert_eq!(
        crate::save_data::SavePayload::from_game_state(&game).unwrap(),
        envelope.payload
    );
    game
}
