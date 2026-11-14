use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use bevy::{camera::ScalingMode, text::FontSource};

use super::*;
use crate::{
    app_state::AppStateTransitionRequest,
    game_state::GameState,
    gameplay_canvas::{GameplayCanvasCamera, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH},
    gameplay_rng::{DEFAULT_GAMEPLAY_SEED, GameplayRng},
    name_entry::{NameEntryConfirmed, NameEntryViewState},
    scenario_new_game_assets::ActiveNewGameInputsStatus,
    scenario_root::ScenarioRoot,
    test_support::headless_title_app_with_asset_base,
};

static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

const INTRO_PATH: &str = "data/dialogue/invented-opening.yaml";
const INTRO: &str = r#"id: invented_opening
type: cutscene
lines:
  - "The invented lantern woke."
  - "unbrokeninventedwordthatmustfallbacktocharacterwrappingwithoutescapingthepanel"
  - "The invented road waited."
on_complete:
  set_flag: invented_intro_completed_later
  transition:
    map: invented_destination
    position: [8, 9]
    fade: in
"#;

struct InventedPackage(PathBuf);

impl InventedPackage {
    fn new(intro: Option<&str>) -> Self {
        let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rpg-s1-intro-dialogue-{}-{unique}",
            std::process::id()
        ));
        let package = root.join("scenarios/invented_campaign");
        fs::create_dir_all(package.join("data/dialogue")).expect("invented dialogue directory");
        let manifest =
            include_str!("../../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
                .replacen("data/dialogue/intro_cutscene.yaml", INTRO_PATH, 1);
        fs::write(package.join("manifest.yaml"), manifest).expect("invented manifest");
        let party = include_str!("../../../../tests/fixtures/party-catalog-shapes.yaml")
            .replacen("id: ember", "id: aric", 1)
            .replacen("class: vanguard", "class: hero", 1);
        fs::write(package.join("data/party.yaml"), party).expect("invented party");
        fs::write(
            package.join("data/balance.yaml"),
            include_str!("../../../../tests/fixtures/balance-complete.yaml"),
        )
        .expect("invented balance");
        if let Some(intro) = intro {
            fs::write(package.join(INTRO_PATH), intro).expect("invented intro");
        }
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for InventedPackage {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("invented package should be removed");
    }
}

fn app_for(package: &InventedPackage, initial_state: AppState) -> App {
    headless_title_app_with_asset_base(
        initial_state,
        package.path().to_string_lossy().into_owned(),
        ScenarioRoot::try_for_package_key("invented_campaign")
            .expect("invented key should be valid"),
    )
}

fn install_and_enter_dialogue(package: &InventedPackage) -> App {
    let mut app = app_for(package, AppState::NameEntry);
    for _ in 0..1_000 {
        app.update();
        if *app.world().resource::<NameEntryViewState>() == NameEntryViewState::Ready
            && app.world().resource::<ActiveNewGameInputs>().status()
                == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(
        app.world().resource::<ActiveNewGameInputs>().status(),
        ActiveNewGameInputsStatus::Ready
    );
    app.world_mut()
        .write_message(NameEntryConfirmed::for_test("Invented Hero"));
    for _ in 0..1_000 {
        app.update();
        if app.world().resource::<State<AppState>>().get() == &AppState::Dialogue
            && *app.world().resource::<IntroDialogueViewState>() == IntroDialogueViewState::Ready
        {
            return app;
        }
        std::thread::yield_now();
    }
    panic!("installed game did not enter a ready intro dialogue");
}

fn marked_text<M: Component>(world: &mut World) -> String {
    world
        .query_filtered::<&Text, With<M>>()
        .single(world)
        .expect("marked text should exist once")
        .0
        .clone()
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

fn release(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
}

fn completions(app: &mut App) -> Vec<IntroDialogueCompleted> {
    app.world_mut()
        .resource_mut::<Messages<IntroDialogueCompleted>>()
        .drain()
        .collect()
}

#[test]
fn empty_progression_completes_once_without_indexing_a_line() {
    let mut progress = IntroDialogueProgress::default();

    assert_eq!(progress.confirm(0), ProgressEffect::Completed);
    assert_eq!(progress.line_index, 0);
    assert!(progress.completed);
    assert_eq!(progress.confirm(0), ProgressEffect::None);
}

#[test]
fn progression_visits_each_middle_line_then_completes_idempotently() {
    let mut progress = IntroDialogueProgress::default();

    assert_eq!(progress.line_index, 0);
    assert_eq!(progress.confirm(3), ProgressEffect::AdvancedTo(1));
    assert_eq!(progress.line_index, 1);
    assert_eq!(progress.confirm(3), ProgressEffect::AdvancedTo(2));
    assert_eq!(progress.line_index, 2);
    assert_eq!(progress.confirm(3), ProgressEffect::Completed);
    assert!(progress.completed);
    assert_eq!(progress.confirm(3), ProgressEffect::None);
    assert_eq!(progress.line_index, 2);
}

#[test]
fn final_confirm_produces_and_applies_only_the_intro_flag_once() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    assert!(
        !app.world()
            .resource::<GameState>()
            .flags()
            .is_set("invented_intro_completed_later")
    );
    for line_index in 0..3 {
        press(&mut app, KeyCode::Enter);
        app.update();
        if line_index < 2 {
            assert!(
                !app.world()
                    .resource::<GameState>()
                    .flags()
                    .is_set("invented_intro_completed_later"),
                "the completion flag must remain absent before the final line"
            );
        }
        // The production InputPlugin clears edge state each frame. This headless fixture
        // injects directly into ButtonInput, so mirror that lifecycle explicitly.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::Enter);
        release(&mut app, KeyCode::Enter);
        app.update();
    }
    assert!(
        app.world()
            .resource::<GameState>()
            .flags()
            .is_set("invented_intro_completed_later")
    );
    let flags = app
        .world()
        .resource::<GameState>()
        .flags()
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    app.update();
    assert_eq!(
        app.world()
            .resource::<GameState>()
            .flags()
            .iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        flags
    );
}

#[test]
fn completion_without_game_state_is_drained_before_a_later_session() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let game = app.world_mut().remove_resource::<GameState>().unwrap();
    let actions: DialogueActions =
        crate::scenario_yaml::from_str("set_flag: should_not_replay").unwrap();
    app.world_mut()
        .resource_mut::<Messages<IntroDialogueCompleted>>()
        .write(IntroDialogueCompleted::for_test(actions));
    app.update();
    app.world_mut().insert_resource(game);
    app.update();
    assert!(
        !app.world()
            .resource::<GameState>()
            .flags()
            .is_set("should_not_replay")
    );
}

#[test]
fn completion_flag_handler_ignores_non_flag_actions_and_keeps_dialogue_state() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let mut expected_rng = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED);
    assert_eq!(
        app.world_mut()
            .resource_mut::<GameState>()
            .rng_mut()
            .next_u64(),
        expected_rng.next_u64()
    );
    let (flags, party, repository, map, opened, controlled, playtime, session_start) = {
        let game = app.world().resource::<GameState>();
        (
            game.flags().clone(),
            game.party().clone(),
            game.repository().clone(),
            game.map().clone(),
            game.opened_boxes().clone(),
            game.controlled_member_id().to_owned(),
            game.playtime().total_seconds(),
            game.playtime().session_start(),
        )
    };
    let actions: DialogueActions = crate::scenario_yaml::from_str("set_flag: only_flag\ngive_items:\n  - id: potion\n    qty: 2\njoin_party: someone\nopen_shop: item\nopen_inn: true\nopen_apothecary: true\n").unwrap();
    app.world_mut()
        .resource_mut::<Messages<IntroDialogueCompleted>>()
        .write(IntroDialogueCompleted::for_test(actions));
    app.update();
    assert_eq!(
        app.world_mut()
            .resource_mut::<GameState>()
            .rng_mut()
            .next_u64(),
        expected_rng.next_u64(),
        "intro completion must not advance the gameplay RNG"
    );
    let game = app.world().resource::<GameState>();
    let mut expected_flags = flags;
    expected_flags.set("only_flag");
    assert_eq!(game.flags(), &expected_flags);
    assert_eq!(game.party(), &party);
    assert_eq!(game.repository(), &repository);
    assert_eq!(game.map(), &map);
    assert_eq!(game.opened_boxes(), &opened);
    assert_eq!(game.controlled_member_id(), controlled);
    assert_eq!(game.playtime().total_seconds(), playtime);
    assert_eq!(game.playtime().session_start(), session_start);
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Dialogue
    );
}

#[test]
fn installed_manifest_intro_uses_one_fixed_canvas_tree_selected_font_and_bounded_wrapping() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let world = app.world_mut();

    assert_eq!(
        marked_text::<IntroDialogueLine>(world),
        "The invented lantern woke."
    );
    assert_eq!(
        world
            .query_filtered::<Entity, With<IntroDialogueCamera>>()
            .iter(world)
            .count(),
        1
    );
    assert_eq!(
        world
            .query_filtered::<Entity, With<IntroDialogueRoot>>()
            .iter(world)
            .count(),
        1
    );
    assert_eq!(
        world
            .query_filtered::<Entity, With<IntroDialoguePanel>>()
            .iter(world)
            .count(),
        1
    );

    let projection = world
        .query_filtered::<&Projection, With<GameplayCanvasCamera>>()
        .single(world)
        .expect("one intro gameplay camera");
    let Projection::Orthographic(projection) = projection else {
        panic!("intro camera should be orthographic");
    };
    let ScalingMode::Fixed { width, height } = projection.scaling_mode else {
        panic!("intro camera should use the fixed logical canvas");
    };
    assert_eq!(
        (width, height),
        (LOGICAL_CANVAS_WIDTH as f32, LOGICAL_CANVAS_HEIGHT as f32,)
    );

    let root = world
        .query_filtered::<&Node, With<IntroDialogueRoot>>()
        .single(world)
        .unwrap();
    assert_eq!(root.width, percent(100));
    assert_eq!(root.height, percent(100));
    assert_eq!(root.justify_content, JustifyContent::Center);
    assert_eq!(root.align_items, AlignItems::Center);

    let panel = world
        .query_filtered::<&Node, With<IntroDialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(panel.width, px(INTRO_PANEL_WIDTH));
    assert_eq!(panel.max_width, px(INTRO_PANEL_WIDTH));
    assert_eq!(panel.height, px(INTRO_PANEL_HEIGHT));
    assert_eq!(panel.max_height, px(INTRO_PANEL_HEIGHT));
    assert!(INTRO_PANEL_WIDTH < LOGICAL_CANVAS_WIDTH as f32);
    assert!(INTRO_PANEL_HEIGHT < LOGICAL_CANVAS_HEIGHT as f32);

    let (layout, node, font) = world
        .query_filtered::<(&TextLayout, &Node, &TextFont), With<IntroDialogueLine>>()
        .single(world)
        .unwrap();
    assert_eq!(layout.linebreak, LineBreak::WordOrCharacter);
    assert_eq!(node.width, px(INTRO_TEXT_WIDTH));
    assert_eq!(node.max_width, px(INTRO_TEXT_WIDTH));
    assert_eq!(node.height, px(INTRO_TEXT_HEIGHT));
    assert_eq!(node.max_height, px(INTRO_TEXT_HEIGHT));
    assert_eq!(node.overflow, Overflow::clip());
    let FontSource::Handle(font) = &font.font else {
        panic!("intro line should use the manifest font handle");
    };
    let font_path = world
        .resource::<AssetServer>()
        .get_path(font.id())
        .expect("intro font should retain its AssetServer path")
        .path()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        font_path,
        "scenarios/invented_campaign/media/fonts/Philosopher-Regular.ttf"
    );

    let hint = world
        .query_filtered::<&Node, With<IntroDialogueHint>>()
        .single(world)
        .unwrap();
    assert_eq!(hint.position_type, PositionType::Absolute);
    assert_eq!(hint.right, px(18));
    assert_eq!(hint.bottom, px(10));
}

#[test]
fn fresh_confirm_during_loading_cannot_skip_the_first_ready_line() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    *app.world_mut().resource_mut::<IntroDialogueViewState>() = IntroDialogueViewState::Loading;
    *app.world_mut().resource_mut::<IntroDialogueProgress>() = IntroDialogueProgress::default();
    press(&mut app, KeyCode::Enter);

    app.update();

    assert_eq!(
        app.world().resource::<IntroDialogueViewState>(),
        &IntroDialogueViewState::Ready
    );
    assert_eq!(
        app.world().resource::<IntroDialogueProgress>(),
        &IntroDialogueProgress::default()
    );
    assert_eq!(
        marked_text::<IntroDialogueLine>(app.world_mut()),
        "The invented lantern woke."
    );
    assert!(completions(&mut app).is_empty());

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Enter);
    app.update();
    assert_eq!(
        marked_text::<IntroDialogueLine>(app.world_mut()),
        "The invented lantern woke."
    );
    assert!(completions(&mut app).is_empty());
}

#[test]
fn back_during_loading_is_drained_and_only_a_fresh_back_cancels() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    *app.world_mut().resource_mut::<IntroDialogueViewState>() = IntroDialogueViewState::Loading;
    press(&mut app, KeyCode::Escape);

    app.update();

    assert_eq!(
        app.world().resource::<IntroDialogueViewState>(),
        &IntroDialogueViewState::Ready
    );
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Dialogue
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Dialogue
    );

    release(&mut app, KeyCode::Escape);
    app.update();
    press(&mut app, KeyCode::Escape);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::World
    );
}

#[test]
fn fresh_enter_space_and_keypad_advance_once_then_complete() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let (party, repository, opened, controlled) = {
        let game = app.world().resource::<GameState>();
        (
            game.party().clone(),
            game.repository().clone(),
            game.opened_boxes().clone(),
            game.controlled_member_id().to_owned(),
        )
    };

    press(&mut app, KeyCode::Enter);
    app.update();
    assert!(marked_text::<IntroDialogueLine>(app.world_mut()).starts_with("unbroken"));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Enter);
    app.update();
    assert!(marked_text::<IntroDialogueLine>(app.world_mut()).starts_with("unbroken"));

    release(&mut app, KeyCode::Enter);
    app.update();
    press(&mut app, KeyCode::Space);
    app.update();
    assert_eq!(
        marked_text::<IntroDialogueLine>(app.world_mut()),
        "The invented road waited."
    );
    assert!(completions(&mut app).is_empty());

    release(&mut app, KeyCode::Space);
    app.update();
    press(&mut app, KeyCode::NumpadEnter);
    app.update();
    let completed = completions(&mut app);
    assert_eq!(completed.len(), 1);
    assert_eq!(
        completed[0]
            .on_complete()
            .set_flag
            .as_ref()
            .expect("invented completion flag")
            .as_slice(),
        ["invented_intro_completed_later"]
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::NumpadEnter);
    app.update();
    release(&mut app, KeyCode::NumpadEnter);
    app.update();
    press(&mut app, KeyCode::NumpadEnter);
    app.update();
    assert!(completions(&mut app).is_empty());

    let game = app.world().resource::<GameState>();
    assert!(game.flags().is_set("invented_intro_completed_later"));
    assert_eq!(game.party(), &party);
    assert_eq!(game.repository(), &repository);
    assert_eq!(game.opened_boxes(), &opened);
    assert_eq!(game.controlled_member_id(), controlled);
    assert_eq!(
        game.map().current().unwrap().as_str(),
        "invented_destination"
    );
    assert_eq!(
        game.map().position(),
        crate::scenario_spatial::Position::new(8, 9)
    );
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::World
    );
}

#[test]
fn same_frame_back_and_confirm_cancels_intro_into_manifest_start_without_effects() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let (start_map, start_position, start_flags) = {
        let game = app.world().resource::<GameState>();
        (
            game.map().current().cloned(),
            game.map().position(),
            game.flags().clone(),
        )
    };

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Escape);
    app.update();

    // The central state request is applied on the following frame. Mirror InputPlugin's
    // edge lifecycle before that frame so these held test keys cannot issue a second cancel.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Escape);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Enter);
    app.update();

    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::World
    );
    let game = app.world().resource::<GameState>();
    assert_eq!(game.map().current(), start_map.as_ref());
    assert_eq!(game.map().position(), start_position);
    assert_eq!(game.flags(), &start_flags);
    assert!(!game.flags().is_set("invented_intro_completed_later"));
    assert!(completions(&mut app).is_empty());

    app.update();
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::World
    );
    assert_eq!(
        app.world().resource::<IntroDialogueProgress>(),
        &IntroDialogueProgress::default()
    );
}

#[test]
fn back_without_an_installed_game_cannot_enter_world_or_replay_later() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let game = app.world_mut().remove_resource::<GameState>().unwrap();
    app.world_mut()
        .resource_mut::<Messages<AppStateTransitionRequest>>()
        .clear();
    press(&mut app, KeyCode::Escape);

    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear_just_pressed(KeyCode::Escape);
    app.world_mut().insert_resource(game);
    app.update();

    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Dialogue
    );
    assert_eq!(
        app.world()
            .resource::<Messages<AppStateTransitionRequest>>()
            .len(),
        0
    );
}

#[test]
fn loading_then_failure_replaces_status_with_one_package_relative_error() {
    let package = InventedPackage::new(None);
    let mut app = app_for(&package, AppState::Dialogue);
    app.update();
    assert_eq!(
        marked_text::<IntroDialogueStatus>(app.world_mut()),
        LOADING_MESSAGE
    );

    for _ in 0..1_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Failed
            && *app.world().resource::<IntroDialogueViewState>() == IntroDialogueViewState::Failed
        {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(
        app.world().resource::<IntroDialogueViewState>(),
        &IntroDialogueViewState::Failed
    );
    let failure = marked_text::<IntroDialogueStatus>(app.world_mut());
    assert!(failure.starts_with(&format!("invented_campaign:{INTRO_PATH}:")));
    assert!(!failure.contains(package.path().to_string_lossy().as_ref()));
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueStatus>>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueLine>>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn revoked_ready_intro_removes_stale_line_and_renders_safe_selected_path() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    let intro_id = app
        .world()
        .resource::<Assets<CutsceneDialogue>>()
        .iter()
        .next()
        .expect("one invented intro")
        .0;
    app.world_mut()
        .resource_mut::<Assets<CutsceneDialogue>>()
        .remove(intro_id);

    app.update();

    assert_eq!(
        app.world().resource::<IntroDialogueViewState>(),
        &IntroDialogueViewState::Failed
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueLine>>()
            .iter(app.world())
            .count(),
        0
    );
    let failure = marked_text::<IntroDialogueStatus>(app.world_mut());
    assert_eq!(
        failure,
        format!("invented_campaign:{INTRO_PATH}: active intro dialogue is unavailable")
    );
    assert!(!failure.contains(package.path().to_string_lossy().as_ref()));
}

#[test]
fn repeated_updates_do_not_respawn_and_exit_cleans_every_owned_entity() {
    let package = InventedPackage::new(Some(INTRO));
    let mut app = install_and_enter_dialogue(&package);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueCamera>>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueRoot>>()
            .iter(app.world())
            .count(),
        1
    );

    app.world_mut()
        .write_message(AppStateTransitionRequest::new(AppState::Title));
    app.update();
    app.update();

    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<IntroDialogueEntity>>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world().resource::<IntroDialogueProgress>(),
        &IntroDialogueProgress::default()
    );
    assert_eq!(
        app.world().resource::<IntroDialogueViewState>(),
        &IntroDialogueViewState::Loading
    );
}

#[test]
fn the_continuation_hint_names_only_keys_that_still_confirm() {
    let defaults = InputBindings::default();
    assert_eq!(
        continuation_hint(0, 3, false, &defaults),
        "ENTER / SPACE  ▶  1/3"
    );
    assert_eq!(
        continuation_hint(2, 3, true, &defaults),
        "Introduction complete"
    );
    let mut rebound = InputBindings::default();
    rebound
        .set_keys(
            crate::input_bindings::BindingTarget::Menu(AppAction::Confirm),
            vec![KeyCode::KeyG],
        )
        .expect("a nonempty row");
    assert_eq!(continuation_hint(0, 3, false, &rebound), "G  ▶  1/3");
}
