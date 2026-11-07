//! Fixed-size celebration for one member; Continue follows battle party order.

use super::*;
use crate::{
    battle::rewards::RewardStatDelta, scenario_class::AbilityKind,
    spell_images::spawn_spell_image_with_fallback,
};

pub(super) const WIDTH: f32 = 660.0;
pub(super) const HEIGHT: f32 = 658.0;

#[derive(Default)]
pub(super) struct LevelMemberArt {
    pub(super) class_name: String,
    pub(super) abilities: BTreeMap<String, LevelAbilityArt>,
}

pub(super) struct LevelAbilityArt {
    pub(super) id: String,
    pub(super) subtitle: String,
}

pub(super) fn member_art(
    rewards: &BattleRewards,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> BTreeMap<String, LevelMemberArt> {
    rewards
        .members
        .iter()
        .filter_map(|member| {
            let runtime = game.party().member(&member.member_id)?;
            let class = catalog.class(runtime.class_id())?;
            let abilities = class
                .abilities
                .iter()
                .filter(|ability| member.learned_abilities.contains(&ability.name))
                .map(|ability| {
                    let kind = match &ability.kind {
                        AbilityKind::Physical(_) => "Physical",
                        AbilityKind::Spell(_) => "Magic",
                        AbilityKind::Heal(_) => "Healing",
                        AbilityKind::Buff(_) | AbilityKind::Debuff(_) => "Support",
                        AbilityKind::Utility(_) => "Utility",
                    };
                    (
                        ability.name.clone(),
                        LevelAbilityArt {
                            id: ability.id.clone(),
                            subtitle: format!("{kind} / {} MP", ability.mp_cost),
                        },
                    )
                })
                .collect();
            Some((
                member.member_id.clone(),
                LevelMemberArt {
                    class_name: class.name.clone(),
                    abilities,
                },
            ))
        })
        .collect()
}

pub(super) fn spawn_level_up_modal(
    commands: &mut Commands,
    rewards: &BattleRewards,
    index: usize,
    font: &Handle<Font>,
    art: &RewardModalArt,
) {
    let Some(member) = rewards.members.get(index) else {
        return;
    };
    let rows = rewards.member_rows();
    let row = &rows[index];
    let details = art.members.get(&member.member_id);
    spawn_reward_frame_node(
        commands,
        Node {
            width: px(WIDTH),
            height: px(HEIGHT),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(24)),
            row_gap: px(16),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            overflow: Overflow::clip(),
            ..default()
        },
        "Level up modal",
        |panel| {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(64),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::SpaceBetween,
                        border: UiRect::bottom(px(1)),
                        ..default()
                    },
                    BorderColor::all(reward_border()),
                ))
                .with_children(|header| {
                    header
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6),
                            ..default()
                        })
                        .with_children(|title| {
                            label(
                                title,
                                "YOUR PARTY GROWS STRONGER",
                                font,
                                10.0,
                                Color::srgb_u8(199, 166, 110),
                            );
                            label(title, "LEVEL UP!", font, 34.0, battle_gold());
                        });
                });
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(14),
                    ..default()
                })
                .with_children(|body| {
                    spawn_identity(
                        body,
                        row,
                        details,
                        art.portraits.get(&member.member_id),
                        font,
                    );
                    body.spawn(Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_height: px(0),
                        column_gap: px(16),
                        ..default()
                    })
                    .with_children(|columns| {
                        columns
                            .spawn(Node {
                                width: px(336),
                                height: percent(100),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                row_gap: px(14),
                                ..default()
                            })
                            .with_children(|stats| {
                                spawn_stat_group(
                                    stats,
                                    "HP & MP",
                                    &row.stats[..2],
                                    336.0,
                                    112.0,
                                    2,
                                    font,
                                );
                                spawn_stat_group(
                                    stats,
                                    "ATTRIBUTES",
                                    &row.stats[2..],
                                    336.0,
                                    184.0,
                                    2,
                                    font,
                                );
                            });
                        spawn_abilities(columns, row, details, art.ability_fallback.as_ref(), font);
                    });
                });
            spawn_footer(panel, font);
        },
    );
}

fn label<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) -> EntityCommands<'a> {
    spawn_battle_text(parent, text, font, size, color, Justify::Left)
}

fn spawn_identity(
    parent: &mut ChildSpawnerCommands<'_>,
    row: &RewardMemberRow,
    details: Option<&LevelMemberArt>,
    portrait: Option<&Handle<Image>>,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(96),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(16),
                ..default()
            },
            Name::new(format!("Level member: {}", row.name)),
        ))
        .with_children(|hero| {
            hero.spawn((
                Node {
                    width: px(78),
                    height: px(96),
                    flex_shrink: 0.0,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(5)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgb_u8(86, 94, 111)),
                BackgroundColor(reward_card()),
            ))
            .with_children(|frame| {
                if let Some(image) = portrait {
                    frame.spawn((
                        RewardPortrait,
                        ImageNode::new(image.clone()).with_mode(NodeImageMode::Stretch),
                        Node {
                            width: percent(100),
                            height: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                }
            });
            hero.spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            })
            .with_children(|identity| {
                label(identity, &row.name, font, 31.0, battle_ink());
                if let Some(details) = details {
                    label(identity, &details.class_name, font, 15.0, reward_muted());
                }
            });
            hero.spawn((
                Node {
                    width: px(148),
                    height: px(96),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: px(8),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(34, 43, 62)),
                BorderColor::all(Color::srgb_u8(86, 94, 111)),
            ))
            .with_children(|tile| {
                tile.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|range| {
                    number(
                        range,
                        format!("Lv{}", row.level_from),
                        font,
                        18.0,
                        Color::srgb_u8(197, 179, 148),
                    );
                    spawn_arrow(range, 18.0);
                    number(
                        range,
                        format!("Lv{}", row.level_to),
                        font,
                        18.0,
                        Color::srgb_u8(224, 207, 170),
                    );
                });
            });
        });
}

fn group_style() -> Node {
    Node {
        height: percent(100),
        flex_shrink: 0.0,
        min_width: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        padding: UiRect::all(px(11)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

fn spawn_stat_group(
    parent: &mut ChildSpawnerCommands<'_>,
    title: &str,
    stats: &[RewardStatDelta],
    width: f32,
    height: f32,
    columns: u16,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_grow: if title == "ATTRIBUTES" { 1.0 } else { 0.0 },
                flex_basis: if title == "ATTRIBUTES" {
                    px(0)
                } else {
                    Val::Auto
                },
                ..group_style()
            },
            BackgroundColor(Color::srgb_u8(25, 32, 49)),
            BorderColor::all(Color::srgb_u8(98, 84, 63)),
            Name::new(format!("Level group: {title}")),
        ))
        .with_children(|group| {
            label(group, title, font, 11.0, Color::srgb_u8(198, 184, 148));
            group
                .spawn(Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(0),
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(columns, 1.0),
                    grid_template_rows: RepeatedGridTrack::flex(
                        if stats.len() > usize::from(columns) {
                            2
                        } else {
                            1
                        },
                        1.0,
                    ),
                    row_gap: px(8),
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|cells| {
                    for stat in stats {
                        spawn_stat(cells, stat, font);
                    }
                });
        });
}

fn number<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) -> EntityCommands<'a> {
    let mut text = label(parent, text, font, size, color);
    text.insert((
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    ));
    text
}

fn spawn_stat(parent: &mut ChildSpawnerCommands<'_>, stat: &RewardStatDelta, font: &Handle<Font>) {
    let previous = stat.total.saturating_sub(stat.gained);
    let long = previous.to_string().len() + stat.total.to_string().len() > 6;
    let title = match stat.label {
        "HP" => "Max HP",
        "MP" => "Max MP",
        "STR" => "Strength",
        "DEX" => "Dexterity",
        "CON" => "Constitution",
        "INT" => "Intelligence",
        other => other,
    };
    parent
        .spawn((
            Node {
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(8), px(6)),
                row_gap: px(5),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(34, 43, 62)),
            BorderColor::all(Color::srgb_u8(66, 76, 96)),
            Name::new(format!("Level stat: {}", stat.label)),
        ))
        .with_children(|card| {
            let title = match stat.label {
                "STR" => "Strength (STR)",
                "DEX" => "Dexterity (DEX)",
                "CON" => "Constitution (CON)",
                "INT" => "Intelligence (INT)",
                _ => title,
            };
            number(card, title, font, 14.0, Color::srgb_u8(183, 185, 189));
            card.spawn(Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(if long { 2.0 } else { 4.0 }),
                ..default()
            })
            .with_children(|values| {
                let old_size = if long { 12.0 } else { 15.0 };
                let new_size = if long { 18.0 } else { 23.0 };
                number(
                    values,
                    previous.to_string(),
                    font,
                    old_size,
                    Color::srgb_u8(150, 159, 170),
                );
                spawn_arrow(values, if long { 10.0 } else { 12.0 });
                number(
                    values,
                    stat.total.to_string(),
                    font,
                    new_size,
                    Color::srgb_u8(236, 196, 119),
                );
                values.spawn(Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                });
                let positive = stat.gained > 0;
                values
                    .spawn((
                        Node {
                            flex_shrink: 0.0,
                            min_width: px(42),
                            justify_content: JustifyContent::Center,
                            padding: UiRect::axes(px(9), px(3)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(999)),
                            ..default()
                        },
                        BackgroundColor(if positive {
                            Color::srgb_u8(39, 61, 60)
                        } else {
                            Color::srgb_u8(48, 56, 71)
                        }),
                        BorderColor::all(if positive {
                            Color::srgb_u8(73, 102, 89)
                        } else {
                            Color::srgb_u8(69, 79, 95)
                        }),
                    ))
                    .with_children(|pill| {
                        number(
                            pill,
                            format!("+{}", stat.gained),
                            font,
                            if long { 10.0 } else { 11.0 },
                            if positive {
                                Color::srgb_u8(160, 219, 189)
                            } else {
                                Color::srgb_u8(137, 145, 158)
                            },
                        );
                    });
            });
        });
}

fn spawn_arrow(parent: &mut ChildSpawnerCommands<'_>, size: f32) {
    let color = Color::srgb_u8(198, 166, 108);
    parent
        .spawn((
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_self: AlignSelf::Center,
                ..default()
            },
            Name::new("Level change arrow"),
        ))
        .with_children(|arrow| {
            arrow.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(size * 0.08),
                    top: px(size * 0.5 - 0.5),
                    width: px(size * 0.8),
                    height: px(1),
                    ..default()
                },
                BackgroundColor(color),
            ));
            arrow.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(size * 0.45),
                    top: px(size * 0.3),
                    width: px(size * 0.4),
                    height: px(size * 0.4),
                    border: UiRect {
                        top: px(1),
                        right: px(1),
                        ..default()
                    },
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BorderColor::all(color),
            ));
        });
}

fn spawn_abilities(
    parent: &mut ChildSpawnerCommands<'_>,
    row: &RewardMemberRow,
    details: Option<&LevelMemberArt>,
    fallback: Option<&Handle<Image>>,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: px(256),
                ..group_style()
            },
            BackgroundColor(Color::srgb_u8(29, 32, 50)),
            BorderColor::all(Color::srgb_u8(97, 81, 114)),
            Name::new("Level group: abilities"),
        ))
        .with_children(|group| {
            label(
                group,
                "NEW ABILITIES",
                font,
                11.0,
                Color::srgb_u8(201, 182, 223),
            );
            super::scroll::spawn_reward_scroll(group, 260.0, |cards| {
                if row.learned_abilities.is_empty() {
                    label(
                        cards,
                        "No new abilities this level.",
                        font,
                        14.0,
                        reward_muted(),
                    );
                }
                for name in &row.learned_abilities {
                    let ability = details.and_then(|details| details.abilities.get(name));
                    cards
                        .spawn((
                            Node {
                                width: percent(100),
                                height: px(58),
                                flex_shrink: 0.0,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(px(6), px(5)),
                                column_gap: px(9),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(5)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(43, 41, 63)),
                            BorderColor::all(Color::srgb_u8(102, 85, 120)),
                            Name::new(format!("Level ability: {name}")),
                        ))
                        .with_children(|card| {
                            spawn_spell_image_with_fallback(
                                card,
                                ability.map_or("", |ability| ability.id.as_str()),
                                40.0,
                                fallback,
                            );
                            card.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                flex_basis: px(0),
                                min_width: px(0),
                                row_gap: px(5),
                                ..default()
                            })
                            .with_children(|text| {
                                label(text, name, font, 16.0, Color::srgb_u8(224, 207, 238));
                                if let Some(ability) = ability {
                                    label(
                                        text,
                                        &ability.subtitle,
                                        font,
                                        11.0,
                                        Color::srgb_u8(170, 155, 185),
                                    );
                                }
                            });
                        });
                }
            });
        });
}
fn spawn_footer(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(58),
                flex_shrink: 0.0,
                padding: UiRect::top(px(14)),
                border: UiRect::top(px(1)),
                justify_content: JustifyContent::End,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(reward_border()),
        ))
        .with_children(|footer| {
            footer
                .spawn((
                    BattleRewardContinue,
                    Node {
                        height: px(42),
                        padding: UiRect::axes(px(17), px(0)),
                        column_gap: px(18),
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(74, 56, 32)),
                    BorderColor::all(Color::srgb_u8(174, 132, 68)),
                ))
                .with_children(|button| {
                    label(button, "Continue", font, 17.0, battle_ink());
                    label(button, "ENTER", font, 11.0, battle_gold());
                });
        });
}
