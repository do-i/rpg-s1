use super::*;
use crate::battle::rewards::{LootReward, MemberReward, RewardStatDelta};
use crate::runtime_member::RuntimeLevelUp;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn level_up(old_level: u32, new_level: u32) -> RuntimeLevelUp {
    RuntimeLevelUp {
        old_level,
        new_level,
        health: 6,
        mana: 3,
        strength: 2,
        dexterity: 1,
        constitution: 2,
        intelligence: 0,
        max_health: 28,
        max_mana: 15,
        total_strength: 30,
        total_dexterity: 18,
        total_constitution: 30,
        total_intelligence: 5,
    }
}

fn rewards_with(members: Vec<MemberReward>) -> BattleRewards {
    BattleRewards {
        total_experience: 120,
        gp_gained: 40,
        members,
        loot: Vec::new(),
        boss_flag: None,
    }
}

fn reward_reveal_app(rewards: BattleRewards) -> App {
    let mut state = crate::battle::tests::state_with(Vec::new());
    state.phase = BattlePhase::Rewards;
    state.rewards = Some(rewards);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<PlaySfx>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)))
        .insert_resource(state)
        .insert_resource(BattleAssetState::test_stub())
        .add_systems(Update, sync_reward_modal);
    app
}

#[test]
fn level_up_replaces_loot_only_after_continue_and_plays_its_own_cue() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "aric".to_owned(),
        member_name: "Aric".to_owned(),
        experience_gained: 40,
        experience_applied: 40,
        experience_now: 450,
        experience_next: 1000,
        level_ups: vec![level_up(1, 2)],
        learned_abilities: Vec::new(),
    }]);
    let mut app = reward_reveal_app(rewards.clone());
    app.update();
    let mut cues = app.world().resource::<Messages<PlaySfx>>().get_cursor();
    assert_eq!(
        cues.read(app.world().resource::<Messages<PlaySfx>>())
            .copied()
            .collect::<Vec<_>>(),
        [PlaySfx::boosted(cue::VICTORY, 220)]
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world_mut()
            .query::<&BattleRewardModal>()
            .iter(app.world())
            .count(),
        0
    );
    app.update();
    assert!(!app.world().contains_resource::<BattleRewardPrelude>());
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Name, With<BattleRewardModal>>()
            .single(app.world())
            .unwrap()
            .as_str(),
        "Victory loot modal"
    );
    assert!(
        cues.read(app.world().resource::<Messages<PlaySfx>>())
            .next()
            .is_none()
    );
    assert!(
        app.world_mut()
            .resource_mut::<BattleRewardScreen>()
            .advance(&rewards)
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Name, With<BattleRewardModal>>()
            .single(app.world())
            .unwrap()
            .as_str(),
        "Level up modal"
    );
    assert_eq!(
        cues.read(app.world().resource::<Messages<PlaySfx>>())
            .copied()
            .collect::<Vec<_>>(),
        [PlaySfx::boosted(cue::LEVEL_UP, 250)]
    );
    app.update();
    assert!(
        cues.read(app.world().resource::<Messages<PlaySfx>>())
            .next()
            .is_none()
    );
    assert!(
        !app.world_mut()
            .resource_mut::<BattleRewardScreen>()
            .advance(&rewards)
    );
    app.world_mut().resource_mut::<BattleState>().phase = BattlePhase::Command;
    app.update();
    assert!(!app.world().contains_resource::<BattleRewardScreen>());
    assert_eq!(
        app.world_mut()
            .query::<&BattleRewardModal>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn rewards_without_a_level_up_wait_for_the_victory_sting() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "aric".to_owned(),
        member_name: "Aric".to_owned(),
        experience_gained: 20,
        experience_applied: 20,
        experience_now: 450,
        experience_next: 1000,
        level_ups: Vec::new(),
        learned_abilities: Vec::new(),
    }]);
    let mut app = reward_reveal_app(rewards);

    app.update();

    assert!(app.world().contains_resource::<BattleRewardPrelude>());
    assert_eq!(
        app.world()
            .resource::<Messages<PlaySfx>>()
            .get_cursor()
            .read(app.world().resource::<Messages<PlaySfx>>())
            .copied()
            .collect::<Vec<_>>(),
        [PlaySfx::boosted(cue::VICTORY, 220)]
    );
    assert_eq!(
        app.world_mut()
            .query::<&BattleRewardModal>()
            .iter(app.world())
            .count(),
        0
    );
    for _ in 0..11 {
        app.update();
    }
    assert!(!app.world().contains_resource::<BattleRewardPrelude>());
    assert_eq!(
        app.world_mut()
            .query::<&BattleRewardModal>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn member_rows_carry_level_range_and_stat_totals_for_a_level_up() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "aric".to_owned(),
        member_name: "Aric".to_owned(),
        experience_gained: 40,
        experience_applied: 40,
        experience_now: 450,
        experience_next: 1000,
        level_ups: vec![level_up(1, 2)],
        learned_abilities: vec!["Power Strike".to_owned()],
    }]);
    let rows = rewards.member_rows();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert!(row.leveled());
    assert_eq!((row.level_from, row.level_to), (1, 2));
    assert_eq!(
        row.stats[0],
        RewardStatDelta {
            label: "HP",
            gained: 6,
            total: 28,
        }
    );
    assert_eq!(row.stats.len(), 6);
    assert_eq!(row.learned_abilities, vec!["Power Strike".to_owned()]);
}

#[test]
fn multi_level_gains_sum_while_totals_track_the_final_level() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "aric".to_owned(),
        member_name: "Aric".to_owned(),
        experience_gained: 500,
        experience_applied: 500,
        experience_now: 450,
        experience_next: 1000,
        level_ups: vec![level_up(1, 2), level_up(2, 3)],
        learned_abilities: Vec::new(),
    }]);
    let row = &rewards.member_rows()[0];
    assert_eq!((row.level_from, row.level_to), (1, 3));
    assert_eq!(row.stats[0].gained, 12);
    assert_eq!(row.stats[0].total, 28);
}

#[test]
fn a_member_without_a_level_up_reports_no_stat_lines() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "elise".to_owned(),
        member_name: "Elise".to_owned(),
        experience_gained: 40,
        experience_applied: 40,
        experience_now: 450,
        experience_next: 1000,
        level_ups: Vec::new(),
        learned_abilities: Vec::new(),
    }]);
    let row = &rewards.member_rows()[0];
    assert!(!row.leveled());
    assert!(row.stats.is_empty());
}

/// Builds the real node tree headlessly and returns every rendered string.
fn rendered_text(rewards: &BattleRewards) -> Vec<String> {
    rendered_screen_text(rewards, BattleRewardScreen::Loot)
}

fn rendered_screen_text(rewards: &BattleRewards, screen: BattleRewardScreen) -> Vec<String> {
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    match screen {
        BattleRewardScreen::Loot => spawn_reward_modal(
            &mut commands,
            rewards,
            &Handle::default(),
            &RewardModalArt::default(),
        ),
        BattleRewardScreen::LevelUp(index) => spawn_level_up_modal(
            &mut commands,
            rewards,
            index,
            &Handle::default(),
            &RewardModalArt::default(),
        ),
    }
    queue.apply(&mut world);
    world
        .query::<&Text>()
        .iter(&world)
        .map(|text| text.0.clone())
        .collect()
}

#[test]
fn loot_and_level_up_screens_show_their_own_reward_details() {
    let mut rewards = rewards_with(vec![
        MemberReward {
            member_id: "aric".to_owned(),
            member_name: "Aric".to_owned(),
            experience_gained: 60,
            experience_applied: 60,
            experience_now: 450,
            experience_next: 1000,
            level_ups: vec![level_up(1, 2)],
            learned_abilities: vec!["Power Strike".to_owned()],
        },
        MemberReward {
            member_id: "elise".to_owned(),
            member_name: "Elise".to_owned(),
            experience_gained: 60,
            experience_applied: 60,
            experience_now: 450,
            experience_next: 1000,
            level_ups: Vec::new(),
            learned_abilities: Vec::new(),
        },
    ]);
    rewards.loot = vec![LootReward {
        item_id: "potion".to_owned(),
        name: "Potion".to_owned(),
        quantity: 2,
        magic_core: false,
    }];
    let text = rendered_text(&rewards);
    for expected in [
        "VICTORY", "40 GP", "Aric", "Elise", "+60 EXP", "Potion", "x2", "Continue", "ENTER",
    ] {
        assert!(text.contains(&expected.to_owned()), "missing {expected}");
    }
    assert!(!text.iter().any(|line| line.contains("LEVEL UP")
        || line.contains("EXP EARNED")
        || line.contains("Learned")));
    let level = rendered_screen_text(&rewards, BattleRewardScreen::LevelUp(0));
    for expected in [
        "LEVEL UP!",
        "Aric",
        "1 > 2",
        "Max HP",
        "+6",
        "28",
        "Intelligence",
        "+0",
        "5",
        "Power Strike",
    ] {
        assert!(level.contains(&expected.to_owned()), "missing {expected}");
    }
    assert!(!level.contains(&"Elise".to_owned()));
    assert!(!level.contains(&"Potion".to_owned()));
}

#[test]
fn a_full_five_member_sweep_still_renders_the_loot_and_the_prompt() {
    let mut rewards = rewards_with(
        ["Aric", "Elise", "Reiya", "Jep", "Kael"]
            .into_iter()
            .map(|name| MemberReward {
                member_id: name.to_lowercase(),
                member_name: name.to_owned(),
                experience_gained: 24,
                experience_applied: 24,
                experience_now: 450,
                experience_next: 1000,
                level_ups: vec![level_up(1, 2)],
                learned_abilities: vec!["Second Wind".to_owned()],
            })
            .collect(),
    );
    rewards.loot = vec![LootReward {
        item_id: "mc_s".to_owned(),
        name: "Magic Core (S)".to_owned(),
        quantity: 1,
        magic_core: true,
    }];
    let text = rendered_text(&rewards);
    for name in ["Aric", "Elise", "Reiya", "Jep", "Kael"] {
        assert!(text.contains(&name.to_owned()), "{name} is missing");
    }
    assert!(text.contains(&"Magic Core (S)".to_owned()));
    assert!(text.contains(&"x1".to_owned()));
    assert!(text.contains(&"Continue".to_owned()));
}

fn layout_app() -> (App, Handle<Font>) {
    use bevy::{asset::AssetPlugin, image::ImagePlugin, text::TextPlugin, ui::UiPlugin};
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        TextPlugin,
        bevy::input::InputPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::window::WindowPlugin {
            primary_window: Some(Window {
                resolution: (1280, 766).into(),
                ..default()
            }),
            ..default()
        },
        bevy::transform::TransformPlugin,
        UiPlugin,
    ))
    .init_asset::<TextureAtlasLayout>()
    .init_resource::<crate::action_input::ActionState>()
    .add_systems(Update, handle_reward_scroll)
    .add_systems(
        PostUpdate,
        update_reward_scroll_chrome.after(bevy::ui::UiSystems::Layout),
    );
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!(scenario_file!("media/fonts/Philosopher-Regular.ttf")).to_vec(),
        ));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: bevy::camera::ComputedCameraValues {
                target_info: Some(bevy::camera::RenderTargetInfo {
                    physical_size: UVec2::new(1280, 766),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
    (app, font)
}

fn full_reward_fixture() -> BattleRewards {
    let mut rewards = rewards_with(
        ["Aric", "Elise", "Reiya", "Jep", "Kael"]
            .into_iter()
            .map(|name| MemberReward {
                member_id: name.to_lowercase(),
                member_name: name.to_owned(),
                experience_gained: 200,
                experience_applied: 200,
                experience_now: 450,
                experience_next: 1000,
                level_ups: vec![level_up(1, 3)],
                learned_abilities: vec!["Radiant Strike".to_owned(), "Kindred Light".to_owned()],
            })
            .collect(),
    );
    rewards.loot = [
        "Magic Core (XS)",
        "Magic Core (S)",
        "Goblin Fang",
        "Goblin Ear",
        "Goblin Shield",
    ]
    .into_iter()
    .map(|name| LootReward {
        item_id: "potion".to_owned(),
        name: name.to_owned(),
        quantity: 2,
        magic_core: false,
    })
    .collect();
    rewards
}

fn spawn_layout(
    app: &mut App,
    font: &Handle<Font>,
    rewards: &BattleRewards,
    screen: BattleRewardScreen,
) {
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    match screen {
        BattleRewardScreen::Loot => {
            spawn_reward_modal(&mut commands, rewards, font, &RewardModalArt::default())
        }
        BattleRewardScreen::LevelUp(index) => spawn_level_up_modal(
            &mut commands,
            rewards,
            index,
            font,
            &RewardModalArt::default(),
        ),
    }
    queue.apply(app.world_mut());
    for _ in 0..5 {
        app.update();
    }
}

#[test]
fn both_reward_screens_fit_five_members_and_scroll_hidden_content() {
    for screen in [BattleRewardScreen::Loot, BattleRewardScreen::LevelUp(0)] {
        let (mut app, font) = layout_app();
        spawn_layout(&mut app, &font, &full_reward_fixture(), screen);
        assert_layout_fits(&mut app);
    }
}

fn assert_layout_fits(app: &mut App) -> (Rect, Vec2) {
    let world = app.world_mut();
    let scrim = world
        .query_filtered::<Entity, With<BattleRewardModal>>()
        .single(world)
        .unwrap();
    let panel = world.get::<Children>(scrim).unwrap()[0];
    let content: Vec<_> = world
        .query_filtered::<Entity, Or<(
            With<Text>,
            With<ItemIcon>,
            With<RewardPortrait>,
            With<crate::spell_images::SpellIcon>,
        )>>()
        .iter(world)
        .collect();
    let rect = |entity: Entity| {
        let node = world.get::<ComputedNode>(entity).unwrap();
        assert!(node.size().min_element() > 0.0, "unmeasured node {entity}");
        Rect::from_center_size(
            world.get::<UiGlobalTransform>(entity).unwrap().translation,
            node.size(),
        )
    };
    let panel_rect = rect(panel);
    assert!(
        panel_rect.min.y >= 0.0 && panel_rect.max.y <= 766.0,
        "{panel_rect:?}"
    );
    for entity in content {
        let bounds = rect(entity);
        let mut ancestor = world.get::<ChildOf>(entity).unwrap().parent();
        loop {
            // Content outside the viewport is intentionally clipped and reached by scrolling.
            if world.get::<scroll::RewardScrollList>(ancestor).is_some() {
                break;
            }
            let ancestor_rect = rect(ancestor).inflate(0.5);
            assert!(
                ancestor_rect.contains(bounds.min) && ancestor_rect.contains(bounds.max),
                "reward content clipped: {:?} {bounds:?} within {ancestor_rect:?}",
                world.get::<Text>(entity)
            );
            if ancestor == panel {
                break;
            }
            ancestor = world.get::<ChildOf>(ancestor).unwrap().parent();
        }
    }
    let button = world
        .query_filtered::<&UiGlobalTransform, With<BattleRewardContinue>>()
        .single(world)
        .unwrap()
        .translation;
    (panel_rect, button)
}

#[test]
fn loot_scroll_clamps_highlights_overflow_and_keeps_party_and_continue_fixed() {
    let (mut app, font) = layout_app();
    spawn_layout(
        &mut app,
        &font,
        &full_reward_fixture(),
        BattleRewardScreen::Loot,
    );
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<scroll::RewardScrollList>>()
        .single(app.world())
        .unwrap();
    let maximum = scroll::max_scroll(app.world().get::<ComputedNode>(list).unwrap());
    assert!(maximum > 0.0);
    let fixed_positions = |world: &mut World| {
        world
            .query::<(&Name, &UiGlobalTransform)>()
            .iter(world)
            .filter(|(name, _)| name.as_str().starts_with("Reward EXP:"))
            .map(|(_, transform)| transform.translation)
            .collect::<Vec<_>>()
    };
    let before = fixed_positions(app.world_mut());
    let confirm = app
        .world_mut()
        .query_filtered::<&UiGlobalTransform, With<BattleRewardContinue>>()
        .single(app.world())
        .unwrap()
        .translation;
    for _ in 0..10 {
        app.world_mut()
            .resource_mut::<crate::action_input::ActionState>()
            .press(crate::action_input::AppAction::Down);
        app.update();
    }
    app.world_mut()
        .resource_mut::<crate::action_input::ActionState>()
        .clear();
    assert!((app.world().get::<ScrollPosition>(list).unwrap().y - maximum).abs() < 0.5);
    assert_eq!(before, fixed_positions(app.world_mut()));
    assert_eq!(
        confirm,
        app.world_mut()
            .query_filtered::<&UiGlobalTransform, With<BattleRewardContinue>>()
            .single(app.world())
            .unwrap()
            .translation
    );
    let frame = app
        .world_mut()
        .query_filtered::<&BorderColor, With<scroll::RewardScrollFrame>>()
        .single(app.world())
        .unwrap();
    assert_eq!(frame.top, battle_gold());
    // The last card must be visible after scrolling to the end.
    let last = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(_, name)| name.as_str() == "Reward loot: potion")
        .last()
        .unwrap()
        .0;
    let node = app.world().get::<ComputedNode>(last).unwrap();
    let center = app
        .world()
        .get::<UiGlobalTransform>(last)
        .unwrap()
        .translation;
    let viewport = app.world().get::<ComputedNode>(list).unwrap();
    let viewport_center = app
        .world()
        .get::<UiGlobalTransform>(list)
        .unwrap()
        .translation;
    assert!(center.y + node.size().y * 0.5 <= viewport_center.y + viewport.size().y * 0.5 + 0.5);
}

#[test]
fn a_single_loot_card_does_not_highlight_or_scroll() {
    let (mut app, font) = layout_app();
    let mut rewards = full_reward_fixture();
    rewards.loot.truncate(1);
    spawn_layout(&mut app, &font, &rewards, BattleRewardScreen::Loot);
    let node = app
        .world_mut()
        .query_filtered::<&ComputedNode, With<scroll::RewardScrollList>>()
        .single(app.world())
        .unwrap();
    assert_eq!(scroll::max_scroll(node), 0.0);
    let frame = app
        .world_mut()
        .query_filtered::<&BorderColor, With<scroll::RewardScrollFrame>>()
        .single(app.world())
        .unwrap();
    assert_eq!(frame.top, reward_border());
}

#[test]
fn an_empty_haul_still_says_so_rather_than_rendering_a_blank_section() {
    let rewards = rewards_with(Vec::new());
    let text = rendered_text(&rewards);
    assert!(text.contains(&"Treasure recovered".to_owned()));
    assert!(text.contains(&"No loot".to_owned()));
}

#[test]
fn experience_label_reveals_the_earned_amount_only_when_capped() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "jep".to_owned(),
        member_name: "Jep".to_owned(),
        experience_gained: 90,
        experience_applied: 30,
        experience_now: 450,
        experience_next: 1000,
        level_ups: Vec::new(),
        learned_abilities: Vec::new(),
    }]);
    let row = &rewards.member_rows()[0];
    assert_eq!(experience_label(row), "+30 EXP  (90 earned)");
    let uncapped = RewardMemberRow {
        experience_gained: 30,
        ..row.clone()
    };
    assert_eq!(experience_label(&uncapped), "+30 EXP");
    assert_eq!(progress_label(row), "EXP 450/1000");
    assert!((progress_fraction(row) - 0.45).abs() < f32::EPSILON);
    let capped = RewardMemberRow {
        experience_next: 0,
        ..row.clone()
    };
    assert_eq!(progress_label(&capped), "MAX LEVEL");
}

#[test]
fn the_modal_uses_only_ascii_text_supported_by_the_scenario_font() {
    let rewards = rewards_with(vec![MemberReward {
        member_id: "aric".to_owned(),
        member_name: "Aric".to_owned(),
        experience_gained: 40,
        experience_applied: 40,
        experience_now: 450,
        experience_next: 1000,
        level_ups: vec![level_up(1, 2)],
        learned_abilities: vec!["Power Strike".to_owned()],
    }]);

    for line in rendered_text(&rewards) {
        assert!(
            line.is_ascii(),
            "unsupported victory-modal glyph in {line:?}"
        );
    }
}

#[test]
fn the_battle_log_spells_out_the_spoils() {
    let mut rewards = rewards_with(vec![
        MemberReward {
            member_id: "aric".to_owned(),
            member_name: "Aric".to_owned(),
            experience_gained: 40,
            experience_applied: 40,
            experience_now: 450,
            experience_next: 1000,
            level_ups: vec![level_up(1, 2)],
            learned_abilities: vec!["Power Strike".to_owned()],
        },
        MemberReward {
            member_id: "jep".to_owned(),
            member_name: "Jep".to_owned(),
            experience_gained: 90,
            experience_applied: 30,
            experience_now: 450,
            experience_next: 1000,
            level_ups: Vec::new(),
            learned_abilities: Vec::new(),
        },
    ]);
    rewards.loot.push(LootReward {
        item_id: "potion".to_owned(),
        name: "Potion".to_owned(),
        quantity: 2,
        magic_core: false,
    });

    assert_eq!(
        rewards.log_lines(),
        [
            "Won 120 EXP and 40 GP.",
            "Aric gains 40 EXP.",
            "Aric reaches level 2! HP +6, MP +3, STR +2, DEX +1, CON +2.",
            "Aric learns Power Strike.",
            "Jep gains 30 EXP (90 earned).",
            "Found Potion x2.",
        ]
    );
}

#[test]
fn wheel_scroll_is_limited_to_the_reward_viewport() {
    use crate::gameplay_canvas::pointer::CanvasPointer;
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
    let (mut app, font) = layout_app();
    spawn_layout(
        &mut app,
        &font,
        &full_reward_fixture(),
        BattleRewardScreen::Loot,
    );
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<scroll::RewardScrollList>>()
        .single(app.world())
        .unwrap();
    let center = app
        .world()
        .get::<UiGlobalTransform>(list)
        .unwrap()
        .translation;
    app.insert_resource(CanvasPointer::at_viewport_position(center, false));
    let wheel = || MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -3.0,
        window: Entity::PLACEHOLDER,
        phase: bevy::input::touch::TouchPhase::Moved,
    };
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(wheel());
    app.update();
    let after = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(after > 0.0);
    app.insert_resource(CanvasPointer::at_viewport_position(Vec2::ZERO, false));
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(wheel());
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, after);
}

#[test]
fn dragging_the_reward_scrollbar_reaches_both_ends_and_stops_on_release() {
    use crate::gameplay_canvas::pointer::CanvasPointer;
    let (mut app, font) = layout_app();
    spawn_layout(
        &mut app,
        &font,
        &full_reward_fixture(),
        BattleRewardScreen::Loot,
    );
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<scroll::RewardScrollList>>()
        .single(app.world())
        .unwrap();
    let (node, transform) = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<scroll::RewardScrollTrack>>()
        .single(app.world())
        .unwrap();
    let top = transform.translation - Vec2::Y * (node.size().y * 0.5 - 1.0);
    let bottom = transform.translation + Vec2::Y * (node.size().y * 0.5 - 1.0);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(bottom));
    app.insert_resource(CanvasPointer::at_viewport_position(bottom, true));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    let maximum = scroll::max_scroll(app.world().get::<ComputedNode>(list).unwrap());
    assert!((app.world().get::<ScrollPosition>(list).unwrap().y - maximum).abs() < 0.5);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(top));
    app.insert_resource(CanvasPointer::at_viewport_position(top, false));
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(bottom));
    app.insert_resource(CanvasPointer::at_viewport_position(bottom, false));
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
}

#[test]
fn sequence_visits_every_leveled_member_in_battle_order_without_repeating() {
    let rewards = full_reward_fixture();
    let mut screen = BattleRewardScreen::Loot;
    for index in 0..5 {
        assert!(screen.advance(&rewards));
        assert_eq!(screen, BattleRewardScreen::LevelUp(index));
        let text = rendered_screen_text(&rewards, screen);
        assert!(text.contains(&rewards.members[index].member_name));
        for (other, member) in rewards.members.iter().enumerate() {
            if other != index {
                assert!(!text.contains(&member.member_name));
            }
        }
    }
    assert!(!screen.advance(&rewards));
    assert_eq!(screen, BattleRewardScreen::LevelUp(4));
}

#[test]
fn sequence_skips_members_without_growth_and_reports_the_filtered_count() {
    let mut rewards = full_reward_fixture();
    for index in [0, 2, 4] {
        rewards.members[index].level_ups.clear();
    }
    let mut screen = BattleRewardScreen::Loot;
    assert!(screen.advance(&rewards));
    assert_eq!(screen, BattleRewardScreen::LevelUp(1));
    let first = rendered_screen_text(&rewards, screen);
    assert!(first.contains(&"1 / 2 members".to_owned()));
    assert!(first.contains(&"2 of 5 leveled up".to_owned()));
    assert!(first.contains(&"Up next Jep".to_owned()));
    assert!(screen.advance(&rewards));
    assert_eq!(screen, BattleRewardScreen::LevelUp(3));
    let last = rendered_screen_text(&rewards, screen);
    assert!(last.contains(&"2 / 2 members".to_owned()));
    assert!(last.contains(&"Return to the field".to_owned()));
    assert!(!screen.advance(&rewards));
    for member in &mut rewards.members {
        member.level_ups.clear();
    }
    assert!(!BattleRewardScreen::Loot.advance(&rewards));
}

#[test]
fn level_up_frame_groups_and_continue_stay_fixed_for_zero_one_or_many_abilities() {
    let mut rewards = full_reward_fixture();
    rewards.members[1].learned_abilities.truncate(1);
    rewards.members[2].learned_abilities.clear();
    rewards.members[3].learned_abilities = (0..12)
        .map(|i| format!("An exceptionally long learned ability name {i}"))
        .collect();
    let mut baseline = None;
    for index in 0..5 {
        let (mut app, font) = layout_app();
        spawn_layout(
            &mut app,
            &font,
            &rewards,
            BattleRewardScreen::LevelUp(index),
        );
        let geometry = assert_layout_fits(&mut app);
        assert_eq!(geometry.0.size(), Vec2::new(level::WIDTH, level::HEIGHT));
        let groups = app
            .world_mut()
            .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
            .iter(app.world())
            .filter(|(name, _, _)| name.as_str().starts_with("Level group:"))
            .map(|(name, node, transform)| {
                (name.as_str().to_owned(), node.size(), transform.translation)
            })
            .collect::<Vec<_>>();
        assert_eq!(groups.len(), 3);
        let current = (geometry, groups);
        if let Some(baseline) = &baseline {
            assert_eq!(baseline, &current);
        } else {
            baseline = Some(current);
        }
        let list_node = app
            .world_mut()
            .query_filtered::<&ComputedNode, With<scroll::RewardScrollList>>()
            .single(app.world())
            .unwrap();
        if index == 3 {
            assert!(scroll::max_scroll(list_node) > 0.0);
        } else {
            assert_eq!(scroll::max_scroll(list_node), 0.0);
        }
    }
}

#[test]
fn large_stat_values_and_max_level_still_fit_inside_the_fixed_frame() {
    let mut rewards = full_reward_fixture();
    let mut level = level_up(99, 100);
    level.max_health = 12_345;
    level.health = 2_346;
    level.max_mana = 9_999;
    level.mana = 1_000;
    level.total_strength = 999;
    level.strength = 100;
    rewards.members[0].level_ups = vec![level];
    let (mut app, font) = layout_app();
    spawn_layout(&mut app, &font, &rewards, BattleRewardScreen::LevelUp(0));
    assert_layout_fits(&mut app);
}

#[test]
fn learned_ability_art_uses_the_members_class_and_stable_ability_id() {
    let rewards = full_reward_fixture();
    let art = level::member_art(
        &rewards,
        &crate::test_support::persistence::fresh_game(),
        &FieldMenuCatalog::production_class_fixture(),
    );
    let hero = &art["aric"];
    assert_eq!(hero.class_name, "Hero");
    assert_eq!(hero.abilities["Radiant Strike"].id, "power_strike");
    assert_eq!(hero.abilities["Kindred Light"].id, "kindred_light");
    assert_eq!(hero.abilities["Kindred Light"].subtitle, "Healing / 4 MP");
}
