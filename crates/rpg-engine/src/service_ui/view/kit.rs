//! Builders shared by the workspace-style services (apothecary and forge).

use super::*;

/// The words and wallet on a workspace header; everything else about the header is common.
pub(super) struct HeaderSpec<'a> {
    pub(super) name: &'a str,
    pub(super) title: &'a str,
    pub(super) subtitle: &'a str,
    /// Whether the title block is preceded by the little ember bars.
    pub(super) bars: bool,
    pub(super) wallet: String,
    pub(super) wallet_color: Color,
}

pub(super) fn spawn_workshop_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    face: Option<DialoguePortrait>,
    spec: HeaderSpec<'_>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(76),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(16),
                padding: UiRect::bottom(px(15)),
                margin: UiRect::bottom(px(16)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(status_border()),
            Name::new(spec.name.to_owned()),
        ))
        .with_children(|header| {
            spawn_keeper_frame(header, font, 60.0, face);
            if spec.bars {
                header
                    .spawn(Node {
                        width: px(12),
                        height: px(42),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|bars| spawn_header_bars(bars, 42.0, 0.0));
            }
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, spec.title, font, 30.0, status_ink());
                    spawn_status_text(title, spec.subtitle, font, 11.0, status_muted());
                });
            header
                .spawn((
                    Node {
                        padding: UiRect::left(px(28)),
                        border: UiRect::left(px(1)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        row_gap: px(3),
                        ..default()
                    },
                    BorderColor::all(status_border()),
                ))
                .with_children(|wallet| {
                    spawn_status_text(wallet, "YOUR COIN", font, 11.0, status_muted());
                    spawn_status_text(wallet, spec.wallet, font, 26.0, spec.wallet_color);
                });
        });
}

/// The row that holds a workspace's panels side by side.
pub(super) fn spawn_workspace(
    parent: &mut ChildSpawnerCommands<'_>,
    name: &str,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(16),
                ..default()
            },
            Name::new(name.to_owned()),
        ))
        .with_children(content);
}

#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a distinct per-service panel difference kept as a parameter"
)]
/// A titled panel. `focus` is `None` for a panel that never takes focus (it then carries no
/// outline at all) and `Some(focused)` for one that does.
pub(super) fn spawn_workshop_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    service: &str,
    node: Node,
    title: &str,
    caption: &str,
    font: &Handle<Font>,
    focus: Option<bool>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    let focused = focus == Some(true);
    let mut panel = parent.spawn((
        Node {
            height: percent(100),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(15)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            overflow: Overflow::clip(),
            ..node
        },
        BackgroundColor(Color::srgba_u8(23, 24, 30, 240)),
        BorderColor::all(if focused {
            status_border_active()
        } else {
            status_border()
        }),
        Name::new(format!("{service} {title}")),
    ));
    if focus.is_some() {
        panel.insert(Outline {
            width: px(2),
            offset: px(0),
            color: if focused {
                status_border_active()
            } else {
                Color::NONE
            },
        });
    }
    panel.with_children(|panel| {
        panel
            .spawn((
                Node {
                    width: percent(100),
                    height: px(27),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Start,
                    border: UiRect::bottom(px(1)),
                    padding: UiRect::bottom(px(10)),
                    margin: UiRect::bottom(px(12)),
                    ..default()
                },
                BorderColor::all(status_border()),
            ))
            .with_children(|heading| {
                spawn_status_text(heading, title, font, 12.0, status_gold());
                spawn_status_text(heading, caption, font, 11.0, status_muted());
            });
        content(panel);
    });
}

/// The one-line key hint under a workspace.
pub(super) fn spawn_hint_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    hint: &str,
) {
    parent
        .spawn(Node {
            width: percent(100),
            margin: UiRect::top(px(14)),
            height: px(20),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|footer| spawn_status_text(footer, hint, font, 12.0, status_muted()));
}
