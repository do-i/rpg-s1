//! Full-screen victory summary: experience, level-up stat changes, and loot.
//!
//! The command panel can only ever show four cramped lines, which buried the
//! level-up payoff. This modal owns the whole canvas for the [`BattlePhase::Rewards`]
//! step so a level-up is impossible to miss.

use bevy::prelude::*;

use super::{
    model::{BattlePhase, BattleState},
    rewards::{BattleRewards, RewardMemberRow},
    ui::{
        BattleAssetState, BattleUi, battle_border_active, battle_dim, battle_gold, battle_ink,
        battle_panel, battle_row, battle_row_border, battle_teal, battle_violet, spawn_battle_text,
    },
};

const MODAL_WIDTH: f32 = 860.0;
const MODAL_Z: i32 = 300;

#[derive(Component)]
pub(super) struct BattleRewardModal;

/// Spawns the modal once rewards are applied and tears it down if the phase
/// leaves [`BattlePhase::Rewards`].
pub(super) fn sync_reward_modal(
    mut commands: Commands,
    state: Option<Res<BattleState>>,
    assets: Option<Res<BattleAssetState>>,
    existing: Query<Entity, With<BattleRewardModal>>,
    mut spawned: Local<bool>,
) {
    let (Some(state), Some(assets)) = (state, assets) else {
        return;
    };
    let showing = state.phase == BattlePhase::Rewards;
    if !showing {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        *spawned = false;
        return;
    }
    let Some(rewards) = state.rewards.as_ref() else {
        return;
    };
    if *spawned {
        return;
    }
    *spawned = true;
    spawn_reward_modal(&mut commands, rewards, &assets.font);
}

fn spawn_reward_modal(commands: &mut Commands, rewards: &BattleRewards, font: &Handle<Font>) {
    let rows = rewards.member_rows();
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
        ))
        .with_children(|scrim| {
            scrim
                .spawn((
                    Node {
                        width: px(MODAL_WIDTH),
                        max_height: percent(94),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        padding: UiRect::all(px(20)),
                        row_gap: px(10),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(8)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(battle_panel()),
                    BorderColor::all(battle_border_active()),
                ))
                .with_children(|panel| {
                    spawn_battle_text(panel, "VICTORY", font, 34.0, battle_gold(), Justify::Center)
                        .insert(Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        });
                    spawn_battle_text(
                        panel,
                        format!(
                            "EXP {}    GP {}",
                            rewards.total_experience, rewards.gp_gained
                        ),
                        font,
                        18.0,
                        battle_teal(),
                        Justify::Center,
                    )
                    .insert(Node {
                        width: percent(100),
                        flex_shrink: 0.0,
                        ..default()
                    });
                    spawn_divider(panel);
                    // The member list is the only part allowed to shrink, so a
                    // five-member sweep can never clip the loot or the prompt.
                    panel
                        .spawn(Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            min_height: px(0),
                            row_gap: px(6),
                            overflow: Overflow::clip(),
                            ..default()
                        })
                        .with_children(|list| {
                            for row in &rows {
                                spawn_member_card(list, row, font);
                            }
                        });
                    spawn_divider(panel);
                    spawn_loot_section(panel, rewards, font);
                    if let Some(flag) = rewards.boss_flag.as_ref() {
                        spawn_battle_text(
                            panel,
                            format!("Boss cleared  ({flag})"),
                            font,
                            14.0,
                            battle_gold(),
                            Justify::Left,
                        );
                    }
                    spawn_battle_text(
                        panel,
                        "Press Enter to continue",
                        font,
                        15.0,
                        battle_dim(),
                        Justify::Center,
                    )
                    .insert(Node {
                        width: percent(100),
                        flex_shrink: 0.0,
                        ..default()
                    });
                });
        });
}

fn spawn_member_card(
    parent: &mut ChildSpawnerCommands<'_>,
    row: &RewardMemberRow,
    font: &Handle<Font>,
) {
    let leveled = row.leveled();
    parent
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                row_gap: px(4),
                padding: UiRect::axes(px(10), px(7)),
                border: UiRect::all(px(if leveled { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(battle_row()),
            BorderColor::all(if leveled {
                battle_gold()
            } else {
                battle_row_border()
            }),
        ))
        .with_children(|card| {
            card.spawn(Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(10),
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
                );
                if leveled {
                    header
                        .spawn((
                            Node {
                                padding: UiRect::axes(px(8), px(2)),
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(battle_gold()),
                        ))
                        .with_children(|badge| {
                            spawn_battle_text(
                                badge,
                                format!("LEVEL UP  {} \u{2192} {}", row.level_from, row.level_to),
                                font,
                                15.0,
                                Color::srgb_u8(24, 20, 12),
                                Justify::Center,
                            );
                        });
                }
                header.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                spawn_battle_text(
                    header,
                    experience_label(row),
                    font,
                    16.0,
                    battle_teal(),
                    Justify::Right,
                );
            });
            if leveled {
                card.spawn(Node {
                    width: percent(100),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(14),
                    row_gap: px(2),
                    ..default()
                })
                .with_children(|stats| {
                    for stat in &row.stats {
                        spawn_battle_text(
                            stats,
                            format!("{} +{} \u{2192} {}", stat.label, stat.gained, stat.total),
                            font,
                            14.0,
                            if stat.gained > 0 {
                                battle_gold()
                            } else {
                                battle_dim()
                            },
                            Justify::Left,
                        );
                    }
                });
            }
            if !row.learned_abilities.is_empty() {
                spawn_battle_text(
                    card,
                    format!("Learned  {}", row.learned_abilities.join(", ")),
                    font,
                    14.0,
                    battle_violet(),
                    Justify::Left,
                );
            }
        });
}

fn spawn_loot_section(
    parent: &mut ChildSpawnerCommands<'_>,
    rewards: &BattleRewards,
    font: &Handle<Font>,
) {
    spawn_battle_text(parent, "LOOT", font, 16.0, battle_gold(), Justify::Left);
    if rewards.loot.is_empty() {
        spawn_battle_text(parent, "No loot", font, 15.0, battle_dim(), Justify::Left);
        return;
    }
    parent
        .spawn(Node {
            width: percent(100),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.0,
            column_gap: px(16),
            row_gap: px(2),
            ..default()
        })
        .with_children(|list| {
            for loot in &rewards.loot {
                spawn_battle_text(
                    list,
                    format!("{}  x{}", loot.name, loot.quantity),
                    font,
                    15.0,
                    if loot.magic_core {
                        battle_violet()
                    } else {
                        battle_ink()
                    },
                    Justify::Left,
                );
            }
        });
}

fn spawn_divider(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
    ));
}

/// Shows the capped amount alongside the earned amount when they disagree.
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
mod tests {
    use super::*;
    use crate::battle::rewards::{LootReward, MemberReward, RewardStatDelta};
    use crate::runtime_member::RuntimeLevelUp;

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

    #[test]
    fn member_rows_carry_level_range_and_stat_totals_for_a_level_up() {
        let rewards = rewards_with(vec![MemberReward {
            member_id: "aric".to_owned(),
            member_name: "Aric".to_owned(),
            experience_gained: 40,
            experience_applied: 40,
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
            level_ups: Vec::new(),
            learned_abilities: Vec::new(),
        }]);
        let row = &rewards.member_rows()[0];
        assert!(!row.leveled());
        assert!(row.stats.is_empty());
    }

    /// Builds the real node tree headlessly and returns every rendered string.
    fn rendered_text(rewards: &BattleRewards) -> Vec<String> {
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        spawn_reward_modal(&mut commands, rewards, &Handle::default());
        queue.apply(&mut world);
        world
            .query::<&Text>()
            .iter(&world)
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn the_modal_renders_the_level_up_badge_stat_deltas_and_loot() {
        let mut rewards = rewards_with(vec![
            MemberReward {
                member_id: "aric".to_owned(),
                member_name: "Aric".to_owned(),
                experience_gained: 60,
                experience_applied: 60,
                level_ups: vec![level_up(1, 2)],
                learned_abilities: vec!["Power Strike".to_owned()],
            },
            MemberReward {
                member_id: "elise".to_owned(),
                member_name: "Elise".to_owned(),
                experience_gained: 60,
                experience_applied: 60,
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
        assert!(text.contains(&"VICTORY".to_owned()));
        assert!(text.contains(&"EXP 120    GP 40".to_owned()));
        assert!(text.contains(&"LEVEL UP  1 \u{2192} 2".to_owned()));
        assert!(text.contains(&"HP +6 \u{2192} 28".to_owned()));
        assert!(text.contains(&"INT +0 \u{2192} 5".to_owned()));
        assert!(text.contains(&"Learned  Power Strike".to_owned()));
        assert!(text.contains(&"Potion  x2".to_owned()));
        assert!(text.contains(&"Press Enter to continue".to_owned()));
        // The member who did not level shows experience but no stat lines.
        assert!(text.contains(&"Elise".to_owned()));
        assert_eq!(
            text.iter()
                .filter(|line| line.starts_with("LEVEL UP"))
                .count(),
            1
        );
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
        assert!(text.contains(&"Magic Core (S)  x1".to_owned()));
        assert!(text.contains(&"Press Enter to continue".to_owned()));
    }

    #[test]
    fn an_empty_haul_still_says_so_rather_than_rendering_a_blank_section() {
        let rewards = rewards_with(Vec::new());
        let text = rendered_text(&rewards);
        assert!(text.contains(&"LOOT".to_owned()));
        assert!(text.contains(&"No loot".to_owned()));
    }

    #[test]
    fn experience_label_reveals_the_earned_amount_only_when_capped() {
        let rewards = rewards_with(vec![MemberReward {
            member_id: "jep".to_owned(),
            member_name: "Jep".to_owned(),
            experience_gained: 90,
            experience_applied: 30,
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
    }
}
