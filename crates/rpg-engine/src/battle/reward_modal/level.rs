//! Fixed-size celebration for one member; Continue follows battle party order.

use super::*;
use crate::{
    battle::rewards::RewardStatDelta, scenario_class::AbilityKind, spell_images::spawn_spell_image,
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
    let leveled = rewards
        .members
        .iter()
        .filter(|member| !member.level_ups.is_empty())
        .count();
    let ordinal = rewards.members[..=index]
        .iter()
        .filter(|member| !member.level_ups.is_empty())
        .count();
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
                    header
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::End,
                            padding: UiRect::top(px(7)),
                            row_gap: px(7),
                            ..default()
                        })
                        .with_children(|progress| {
                            label(
                                progress,
                                format!("{ordinal} / {leveled} members"),
                                font,
                                15.0,
                                Color::srgb_u8(221, 190, 131),
                            );
                            label(
                                progress,
                                format!("{leveled} of {} leveled up", rewards.members.len()),
                                font,
                                12.0,
                                reward_muted(),
                            );
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
                        height: px(204),
                        flex_shrink: 0.0,
                        column_gap: px(12),
                        ..default()
                    })
                    .with_children(|growth| {
                        spawn_stat_group(growth, "HP & MP", &row.stats[..2], 190.0, 1, font);
                        spawn_stat_group(growth, "ATTRIBUTES", &row.stats[2..], 406.0, 2, font);
                    });
                    spawn_abilities(body, row, details, font);
                });
            let next = rewards
                .members
                .iter()
                .skip(index + 1)
                .find(|member| !member.level_ups.is_empty());
            spawn_footer(panel, next.map(|member| member.member_name.as_str()), font);
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
                BorderColor::all(Color::srgb_u8(201, 159, 87)),
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
                    width: px(120),
                    height: px(96),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: px(10),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(74, 56, 32)),
                BorderColor::all(Color::srgb_u8(162, 126, 64)),
            ))
            .with_children(|tile| {
                label(tile, "LEVEL UP", font, 10.0, Color::srgb_u8(225, 191, 128));
                label(
                    tile,
                    format!("{} > {}", row.level_from, row.level_to),
                    font,
                    32.0,
                    Color::srgb_u8(244, 204, 120),
                );
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
    columns: u16,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: px(width),
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
                    grid_template_rows: RepeatedGridTrack::flex(2, 1.0),
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
                padding: UiRect::axes(px(10), px(9)),
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(34, 43, 62)),
            BorderColor::all(Color::srgb_u8(66, 76, 96)),
            Name::new(format!("Level stat: {}", stat.label)),
        ))
        .with_children(|card| {
            number(card, title, font, 12.0, Color::srgb_u8(183, 185, 189));
            card.spawn(Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(if long { 3.0 } else { 7.0 }),
                ..default()
            })
            .with_children(|values| {
                let old_size = if long { 13.0 } else { 17.0 };
                let new_size = if long { 20.0 } else { 26.0 };
                number(
                    values,
                    previous.to_string(),
                    font,
                    old_size,
                    Color::srgb_u8(150, 159, 170),
                );
                number(values, ">", font, 13.0, Color::srgb_u8(146, 137, 120));
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
                            padding: UiRect::axes(px(if long { 5.0 } else { 7.0 }), px(5)),
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
                            if long { 11.0 } else { 13.0 },
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

fn spawn_abilities(
    parent: &mut ChildSpawnerCommands<'_>,
    row: &RewardMemberRow,
    details: Option<&LevelMemberArt>,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(124),
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
            scroll::spawn_reward_scroll_plain(group, 76.0, |viewport| {
                if row.learned_abilities.is_empty() {
                    label(
                        viewport,
                        "No new abilities this level.",
                        font,
                        14.0,
                        reward_muted(),
                    );
                    return;
                }
                viewport
                    .spawn(Node {
                        width: percent(100),
                        flex_shrink: 0.0,
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Start,
                        column_gap: px(7),
                        row_gap: px(7),
                        ..default()
                    })
                    .with_children(|cards| {
                        for name in &row.learned_abilities {
                            let ability = details.and_then(|details| details.abilities.get(name));
                            cards
                                .spawn((
                                    Node {
                                        min_width: px(0),
                                        max_width: percent(100),
                                        min_height: px(54),
                                        align_items: AlignItems::Center,
                                        padding: UiRect::axes(px(6), px(5)),
                                        column_gap: px(9),
                                        border: UiRect::all(px(1)),
                                        border_radius: BorderRadius::all(px(5)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb_u8(43, 41, 63)),
                                    BorderColor::all(Color::srgb_u8(102, 85, 120)),
                                ))
                                .with_children(|card| {
                                    if let Some(ability) = ability {
                                        spawn_spell_image(card, &ability.id, 40.0);
                                    }
                                    card.spawn(Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        min_width: px(0),
                                        row_gap: px(5),
                                        ..default()
                                    })
                                    .with_children(|text| {
                                        label(
                                            text,
                                            name,
                                            font,
                                            16.0,
                                            Color::srgb_u8(224, 207, 238),
                                        );
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
        });
}

fn spawn_footer(parent: &mut ChildSpawnerCommands<'_>, next: Option<&str>, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(58),
                flex_shrink: 0.0,
                padding: UiRect::top(px(14)),
                border: UiRect::top(px(1)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(reward_border()),
        ))
        .with_children(|footer| {
            label(
                footer,
                next.map_or_else(
                    || "Return to the field".to_owned(),
                    |name| format!("Up next {name}"),
                ),
                font,
                13.0,
                reward_muted(),
            );
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
