//! Victory loot first, followed by a dedicated level-up celebration when needed.

use std::collections::BTreeMap;

use bevy::{ecs::system::SystemParam, prelude::*};

mod level;
mod scroll;
use level::spawn_level_up_modal;
pub(super) use scroll::{handle_reward_scroll, update_reward_scroll_chrome};

use super::{
    model::{BattlePhase, BattleState},
    rewards::{BattleRewards, RewardMemberRow},
    ui::{
        BattleAssetState, BattleUi, battle_border_active, battle_gold, battle_ink, battle_teal,
        spawn_battle_text,
    },
};
use crate::{
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    item_images::ItemIcon,
    options_store::PlayerOptions,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    sfx_cue::{PlaySfx, cue},
    sound_option::channel_gain,
};

const LOOT_MODAL_WIDTH: f32 = 1020.0;
const LOOT_ICON_SIZE: f32 = 54.0;
// Five cards and four 8px gaps fit the viewport's 362px inner height.
const LOOT_CARD_HEIGHT: f32 = 66.0;
const LOOT_VIEW_HEIGHT: f32 = 380.0;
const PARTY_COLUMN_WIDTH: f32 = 340.0;
const MODAL_Z: i32 = 300;
/// Matches the authored victory sample.
const VICTORY_PRELUDE_SECONDS: f32 = 8.0 / 3.0;

#[derive(Component)]
pub(super) struct BattleRewardModal;

#[derive(Component)]
pub(super) struct BattleRewardContinue;

#[derive(Component)]
pub(super) struct RewardPortrait;

#[derive(Default)]
struct RewardModalArt {
    gold: Option<Handle<Image>>,
    ability_fallback: Option<Handle<Image>>,
    portraits: BTreeMap<String, Handle<Image>>,
    members: BTreeMap<String, level::LevelMemberArt>,
}

impl RewardModalArt {
    fn load(
        server: Option<&AssetServer>,
        root: Option<&ScenarioRoot>,
        rewards: &BattleRewards,
    ) -> Self {
        let (Some(server), Some(root)) = (server, root) else {
            return Self::default();
        };
        let load = |path: &str| {
            ScenarioRelativePath::try_from(path)
                .ok()
                .map(|path| server.load(root.resolve(&path)))
        };
        Self {
            members: BTreeMap::new(),
            gold: load("media/images/icons/stats/gold.png"),
            ability_fallback: load("media/images/icons/menu/spells.png"),
            portraits: rewards
                .members
                .iter()
                .filter_map(|member| {
                    load(&format!(
                        "media/images/party_portraits_large/{}_status_portrait.webp",
                        member.member_id
                    ))
                    .map(|handle| (member.member_id.clone(), handle))
                })
                .collect(),
        }
    }
}

fn reward_panel() -> Color {
    Color::srgb_u8(21, 27, 43)
}
fn reward_card() -> Color {
    Color::srgb_u8(32, 40, 59)
}
fn reward_border() -> Color {
    Color::srgb_u8(121, 97, 58)
}

/// The reward screens advance only on a fresh confirm; rewards are already banked.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum BattleRewardScreen {
    #[default]
    Loot,
    LevelUp(usize),
}

impl BattleRewardScreen {
    /// Returns true when another reward screen replaces the current one.
    pub(super) fn advance(&mut self, rewards: &BattleRewards) -> bool {
        let start = match *self {
            Self::Loot => 0,
            Self::LevelUp(index) => index.saturating_add(1),
        };
        let next = rewards
            .members
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, member)| !member.level_ups.is_empty())
            .map(|(index, _)| index);
        if let Some(index) = next {
            *self = Self::LevelUp(index);
            true
        } else {
            false
        }
    }
}

/// Locks dismissal until the victory sting finishes and the loot is revealed. Confirm cuts it
/// short, and it is never inserted while sound effects are muted, since there is no sting to wait
/// for.
#[derive(Resource)]
pub(super) struct BattleRewardPrelude {
    timer: Timer,
    skipped: bool,
}

impl BattleRewardPrelude {
    pub(super) fn new() -> Self {
        Self {
            timer: Timer::from_seconds(VICTORY_PRELUDE_SECONDS, TimerMode::Once),
            skipped: false,
        }
    }

    /// Reveals the loot on the next modal sync instead of waiting out the sting.
    pub(super) fn skip(&mut self) {
        self.skipped = true;
    }

    #[cfg(test)]
    pub(super) const fn is_skipped(&self) -> bool {
        self.skipped
    }
}

#[derive(SystemParam)]
pub(super) struct RewardModalContext<'w> {
    state: Option<Res<'w, BattleState>>,
    assets: Option<Res<'w, BattleAssetState>>,
    prelude: Option<ResMut<'w, BattleRewardPrelude>>,
    screen: Option<Res<'w, BattleRewardScreen>>,
    time: Res<'w, Time>,
    server: Option<Res<'w, AssetServer>>,
    root: Option<Res<'w, ScenarioRoot>>,
    game: Option<Res<'w, GameState>>,
    catalog: Option<Res<'w, FieldMenuCatalog>>,
    options: Option<Res<'w, PlayerOptions>>,
}

/// Spawns the modal once rewards are applied and tears it down if the phase
/// leaves [`BattlePhase::Rewards`].
pub(super) fn sync_reward_modal(
    mut commands: Commands,
    mut context: RewardModalContext,
    existing: Query<Entity, With<BattleRewardModal>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut spawned: Local<Option<BattleRewardScreen>>,
) {
    let (Some(state), Some(assets)) = (context.state.as_deref(), context.assets.as_deref()) else {
        *spawned = None;
        return;
    };
    let showing = state.phase == BattlePhase::Rewards;
    if !showing {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<BattleRewardPrelude>();
        commands.remove_resource::<BattleRewardScreen>();
        *spawned = None;
        return;
    }
    let Some(rewards) = state.rewards.as_ref() else {
        return;
    };
    let screen = context.screen.as_deref().copied().unwrap_or_default();
    if context.screen.is_none() {
        commands.insert_resource(screen);
    }
    let sfx_muted = context
        .options
        .as_deref()
        .is_some_and(|options| channel_gain(options, false, false) == 0.0);
    if spawned.is_none() && context.prelude.is_none() && !sfx_muted {
        commands.insert_resource(BattleRewardPrelude::new());
        sfx.write(PlaySfx::boosted(cue::VICTORY, 220));
        return;
    }
    if let Some(prelude) = context.prelude.as_deref_mut() {
        if !prelude.skipped && !prelude.timer.tick(context.time.delta()).just_finished() {
            return;
        }
        commands.remove_resource::<BattleRewardPrelude>();
    }
    if *spawned == Some(screen) {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let mut art = RewardModalArt::load(context.server.as_deref(), context.root.as_deref(), rewards);
    if let (Some(game), Some(catalog)) = (context.game.as_deref(), context.catalog.as_deref()) {
        art.members = level::member_art(rewards, game, catalog);
    }
    match screen {
        BattleRewardScreen::Loot => spawn_reward_modal(&mut commands, rewards, &assets.font, &art),
        BattleRewardScreen::LevelUp(index) => {
            spawn_level_up_modal(&mut commands, rewards, index, &assets.font, &art);
            sfx.write(PlaySfx::boosted(cue::LEVEL_UP, 250));
        }
    }
    *spawned = Some(screen);
}

/// Match the preview's top-aligned portrait crop without assuming source image dimensions.
pub(super) fn sync_reward_portraits(
    images: Option<Res<Assets<Image>>>,
    mut portraits: Query<(&mut ImageNode, &ComputedNode), With<RewardPortrait>>,
) {
    let Some(images) = images else { return };
    for (mut image, node) in &mut portraits {
        let Some(source) = images.get(&image.image) else {
            continue;
        };
        let size = source.size().as_vec2();
        let ratio = if node.size().y > 0.0 {
            node.size().x / node.size().y
        } else {
            60.0 / 68.0
        };
        let width = size.x.min(size.y * ratio);
        let height = width / ratio;
        let min = Vec2::new((size.x - width) * 0.5, 0.0);
        let rect = Rect::from_corners(min, min + Vec2::new(width, height));
        if image.rect != Some(rect) {
            image.rect = Some(rect);
        }
    }
}

fn spawn_reward_frame(
    commands: &mut Commands,
    width: f32,
    name: &'static str,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    spawn_reward_frame_node(
        commands,
        Node {
            width: px(width),
            max_height: percent(94),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            padding: UiRect::all(px(26)),
            row_gap: px(18),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            overflow: Overflow::clip(),
            ..default()
        },
        name,
        content,
    );
}

fn spawn_reward_frame_node(
    commands: &mut Commands,
    panel: Node,
    name: &'static str,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.74)),
            GlobalZIndex(MODAL_Z),
            Pickable::IGNORE,
            BattleUi,
            BattleRewardModal,
            Name::new(name),
        ))
        .with_children(|scrim| {
            scrim
                .spawn((
                    panel,
                    BackgroundColor(reward_panel()),
                    BorderColor::all(battle_border_active()),
                ))
                .with_children(content);
        });
}

fn spawn_continue_prompt(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, notice: &str) {
    spawn_divider(parent);
    parent
        .spawn(Node {
            width: percent(100),
            flex_shrink: 0.0,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|footer| {
            spawn_battle_text(footer, notice, font, 13.0, reward_muted(), Justify::Left);
            footer
                .spawn((
                    BattleRewardContinue,
                    Node {
                        padding: UiRect::axes(px(17), px(11)),
                        column_gap: px(18),
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(74, 56, 32)),
                    BorderColor::all(Color::srgb_u8(174, 132, 68)),
                ))
                .with_children(|button| {
                    spawn_battle_text(button, "Continue", font, 17.0, battle_ink(), Justify::Left);
                    spawn_battle_text(button, "ENTER", font, 11.0, battle_gold(), Justify::Center);
                });
        });
}

fn spawn_reward_modal(
    commands: &mut Commands,
    rewards: &BattleRewards,
    font: &Handle<Font>,
    art: &RewardModalArt,
) {
    let rows = rewards.member_rows();
    spawn_reward_frame(commands, LOOT_MODAL_WIDTH, "Victory loot modal", |panel| {
        panel
            .spawn(Node {
                width: percent(100),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|header| {
                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    })
                    .with_children(|title| {
                        spawn_battle_text(
                            title,
                            "BATTLE COMPLETE",
                            font,
                            11.0,
                            battle_gold(),
                            Justify::Left,
                        );
                        spawn_battle_text(
                            title,
                            "VICTORY",
                            font,
                            42.0,
                            battle_gold(),
                            Justify::Left,
                        );
                    });
                header
                    .spawn((
                        Node {
                            padding: UiRect::axes(px(16), px(7)),
                            column_gap: px(12),
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(reward_card()),
                        BorderColor::all(reward_border()),
                    ))
                    .with_children(|gp| {
                        let mut coin = gp.spawn((
                            Node {
                                width: px(48),
                                height: px(48),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Name::new("Reward gold icon"),
                        ));
                        if let Some(image) = &art.gold {
                            coin.insert(ImageNode::new(image.clone()));
                        }
                        spawn_battle_text(
                            gp,
                            format!("{} GP", rewards.gp_gained),
                            font,
                            30.0,
                            battle_gold(),
                            Justify::Left,
                        );
                    });
            });
        spawn_divider(panel);
        panel
            .spawn(Node {
                width: percent(100),
                flex_shrink: 0.0,
                column_gap: px(24),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(PARTY_COLUMN_WIDTH),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                })
                .with_children(|party| {
                    spawn_section_title(party, "Party experience", font);
                    party
                        .spawn(Node {
                            height: px(LOOT_VIEW_HEIGHT),
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(5),
                            ..default()
                        })
                        .with_children(|list| {
                            for (row, member) in rows.iter().zip(&rewards.members) {
                                spawn_party_experience(
                                    list,
                                    row,
                                    art.portraits.get(&member.member_id),
                                    font,
                                );
                            }
                        });
                });
                body.spawn((
                    Node {
                        width: px(1),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(reward_border()),
                ));
                body.spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                })
                .with_children(|loot| {
                    loot.spawn(Node {
                        width: percent(100),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|heading| {
                        spawn_section_title(heading, "Treasure recovered", font);
                        spawn_battle_text(
                            heading,
                            format!("{} item types", rewards.loot.len()),
                            font,
                            12.0,
                            reward_muted(),
                            Justify::Right,
                        );
                    });
                    scroll::spawn_reward_scroll(loot, LOOT_VIEW_HEIGHT, |list| {
                        spawn_loot_section(list, rewards, font)
                    });
                });
            });
        spawn_continue_prompt(panel, font, "Rewards added to your inventory");
    });
}

fn spawn_section_title(parent: &mut ChildSpawnerCommands<'_>, title: &str, font: &Handle<Font>) {
    spawn_battle_text(
        parent,
        title,
        font,
        12.0,
        Color::srgb_u8(201, 185, 143),
        Justify::Left,
    )
    .insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
}

fn spawn_party_experience(
    parent: &mut ChildSpawnerCommands<'_>,
    row: &RewardMemberRow,
    portrait: Option<&Handle<Image>>,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(72),
                flex_shrink: 0.0,
                column_gap: px(14),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new(format!("Reward EXP: {}", row.name)),
        ))
        .with_children(|member| {
            member
                .spawn((
                    Node {
                        width: px(60),
                        height: px(68),
                        flex_shrink: 0.0,
                        overflow: Overflow::clip(),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(reward_card()),
                    BorderColor::all(reward_border()),
                ))
                .with_children(|frame| {
                    if let Some(portrait) = portrait {
                        frame.spawn((
                            RewardPortrait,
                            ImageNode::new(portrait.clone()).with_mode(NodeImageMode::Stretch),
                            Node {
                                width: percent(100),
                                height: percent(100),
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                    }
                });
            member
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    ..default()
                })
                .with_children(|detail| {
                    detail
                        .spawn(Node {
                            width: percent(100),
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|header| {
                            spawn_battle_text(
                                header,
                                row.name.clone(),
                                font,
                                19.0,
                                battle_ink(),
                                Justify::Left,
                            )
                            .insert(Node {
                                flex_grow: 1.0,
                                ..default()
                            });
                            spawn_battle_text(
                                header,
                                experience_label(row),
                                font,
                                17.0,
                                battle_teal(),
                                Justify::Right,
                            );
                        });
                    spawn_battle_text(
                        detail,
                        progress_label(row),
                        font,
                        11.0,
                        reward_muted(),
                        Justify::Right,
                    )
                    .insert(Node {
                        width: percent(100),
                        ..default()
                    });
                    spawn_experience_meter(detail, row);
                });
        });
}

fn spawn_experience_meter(parent: &mut ChildSpawnerCommands<'_>, row: &RewardMemberRow) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(4),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(255, 255, 255, 28)),
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    width: percent(progress_fraction(row) * 100.0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(battle_teal()),
            ));
        });
}

fn reward_muted() -> Color {
    Color::srgb_u8(163, 158, 145)
}

fn spawn_loot_section(
    parent: &mut ChildSpawnerCommands<'_>,
    rewards: &BattleRewards,
    font: &Handle<Font>,
) {
    if rewards.loot.is_empty() {
        spawn_battle_text(parent, "No loot", font, 20.0, reward_muted(), Justify::Left);
    }
    if rewards.shattered_cores > 0 {
        spawn_battle_text(
            parent,
            if rewards.shattered_cores == 1 {
                "Core shattered".to_owned()
            } else {
                format!("{} cores shattered", rewards.shattered_cores)
            },
            font,
            16.0,
            Color::srgb_u8(195, 172, 216),
            Justify::Left,
        );
    }
    for loot in &rewards.loot {
        parent
            .spawn((
                Node {
                    width: percent(100),
                    height: px(LOOT_CARD_HEIGHT),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    padding: UiRect {
                        left: px(8),
                        right: px(12),
                        top: px(5),
                        bottom: px(5),
                    },
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(reward_card()),
                BorderColor::all(Color::srgb_u8(138, 108, 62)),
                Name::new(format!("Reward loot: {}", loot.item_id)),
            ))
            .with_children(|card| {
                card.spawn((
                    ItemIcon::new(&loot.item_id),
                    Node {
                        width: px(LOOT_ICON_SIZE),
                        height: px(LOOT_ICON_SIZE),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
                card.spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|info| {
                    spawn_battle_text(
                        info,
                        loot.name.clone(),
                        font,
                        22.0,
                        battle_ink(),
                        Justify::Left,
                    );
                    spawn_battle_text(
                        info,
                        if loot.magic_core {
                            "Magic core"
                        } else {
                            "Battle treasure"
                        },
                        font,
                        14.0,
                        if loot.magic_core {
                            Color::srgb_u8(195, 172, 216)
                        } else {
                            reward_muted()
                        },
                        Justify::Left,
                    );
                });
                card.spawn((
                    Node {
                        min_width: px(52),
                        padding: UiRect::all(px(6)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(5)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(49, 48, 47)),
                    BorderColor::all(reward_border()),
                ))
                .with_children(|qty| {
                    spawn_battle_text(
                        qty,
                        format!("x{}", loot.quantity),
                        font,
                        22.0,
                        battle_gold(),
                        Justify::Center,
                    );
                });
            });
    }
}

fn spawn_divider(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(divider_color()),
    ));
}

fn divider_color() -> Color {
    Color::srgba_u8(126, 98, 55, 150)
}

fn progress_label(row: &RewardMemberRow) -> String {
    if row.experience_next == 0 {
        "MAX LEVEL".to_owned()
    } else {
        format!("EXP {}/{}", row.experience_now, row.experience_next)
    }
}

fn progress_fraction(row: &RewardMemberRow) -> f32 {
    if row.experience_next == 0 {
        1.0
    } else {
        (row.experience_now as f32 / row.experience_next as f32).clamp(0.0, 1.0)
    }
}

fn experience_label(row: &RewardMemberRow) -> String {
    if row.experience_applied == row.experience_gained {
        format!("+{} EXP", row.experience_applied)
    } else {
        format!(
            "+{} EXP  ({} earned)",
            row.experience_applied, row.experience_gained
        )
    }
}

#[cfg(test)]
mod tests;
