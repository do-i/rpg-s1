//! Durable-write feedback with explicit pointer controls; no gameplay binding is intercepted.

use bevy::{
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform, UiSystems, px},
};

use crate::{
    gameplay_canvas::pointer::CanvasPointer,
    options_store::{
        OptionsLoadStatus, OptionsStore, OptionsStoreError, PlayerOptions, load_player_options,
    },
};

const SUCCESS_NOTICE_SECONDS: f32 = 3.0;

#[derive(Debug, Default)]
pub(crate) struct SaveNotice {
    pub(crate) message: String,
    pub(crate) failed: bool,
    pub(crate) retry_requested: bool,
    dismissed: bool,
    expires_after: Option<Timer>,
}

impl SaveNotice {
    pub(crate) fn report(&mut self, message: String, failed: bool) {
        *self = Self {
            message,
            failed,
            expires_after: (!failed)
                .then(|| Timer::from_seconds(SUCCESS_NOTICE_SECONDS, TimerMode::Once)),
            ..default()
        };
    }

    fn visible(&self) -> bool {
        !self.dismissed && !self.message.is_empty()
    }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct PersistenceStatus {
    pub(crate) autosave: SaveNotice,
    pub(crate) options: SaveNotice,
}

/// Both Options and the battle menu use this path, including the rejected-file guard.
pub(crate) fn persist_player_options(
    store: &OptionsStore,
    options: &PlayerOptions,
    load: &mut OptionsLoadStatus,
    notice: &mut SaveNotice,
) {
    match store.persist(options, load) {
        Ok(_) => {
            *load = OptionsLoadStatus::Loaded;
            notice.report(
                "Options changes are active and saved to disk.".to_owned(),
                false,
            );
        }
        Err(error) => {
            error!(
                "could not save options to {}: {error}",
                store.path().display()
            );
            if !matches!(load, OptionsLoadStatus::Rejected(_))
                && let (_, rejected @ OptionsLoadStatus::Rejected(_)) = load_player_options(store)
            {
                *load = rejected;
            }
            let message = if matches!(
                &error,
                OptionsStoreError::Io {
                    operation: "sync config directory",
                    ..
                }
            ) {
                format!(
                    "Options changes are active and the file was published, but disk durability could not be confirmed: {error}. Retry after fixing the storage problem."
                )
            } else {
                load.rejection_message().unwrap_or_else(|| format!(
                "Options changes are active for this session, but could not be saved to disk: {error}. Retry after fixing the storage problem."
            ))
            };
            notice.report(message, true);
        }
    }
}

#[derive(Component, Clone, Copy)]
enum NoticeKind {
    Autosave,
    Options,
}

impl NoticeKind {
    fn notice(self, status: &PersistenceStatus) -> &SaveNotice {
        match self {
            Self::Autosave => &status.autosave,
            Self::Options => &status.options,
        }
    }
    fn notice_mut(self, status: &mut PersistenceStatus) -> &mut SaveNotice {
        match self {
            Self::Autosave => &mut status.autosave,
            Self::Options => &mut status.options,
        }
    }
}

#[derive(Component)]
struct NoticePanel(NoticeKind);
#[derive(Component)]
struct NoticeText(NoticeKind);
#[derive(Component)]
struct NoticeControl {
    kind: NoticeKind,
    retry: bool,
}

pub(crate) struct PersistenceStatusPlugin;
impl Plugin for PersistenceStatusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersistenceStatus>()
            .init_resource::<Time<Real>>()
            .add_systems(Startup, setup_status_ui)
            .add_systems(
                PreUpdate,
                (expire_success_notices, handle_status_pointer)
                    .chain()
                    .after(crate::gameplay_canvas::pointer::update_canvas_pointer),
            )
            .add_systems(Update, retry_options)
            .add_systems(PostUpdate, refresh_status_ui.before(UiSystems::Prepare));
    }
}

fn expire_success_notices(time: Res<Time<Real>>, mut status: ResMut<PersistenceStatus>) {
    let status = &mut *status;
    for notice in [&mut status.autosave, &mut status.options] {
        if notice.failed || !notice.visible() {
            continue;
        }
        if let Some(timer) = notice.expires_after.as_mut()
            && timer.tick(time.delta()).is_finished()
        {
            notice.dismissed = true;
        }
    }
}

fn setup_status_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                bottom: px(12),
                width: px(480),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            GlobalZIndex(1000),
        ))
        .with_children(|root| {
            for kind in [NoticeKind::Autosave, NoticeKind::Options] {
                root.spawn((
                    Node {
                        display: Display::None,
                        padding: UiRect::all(px(10)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(22, 22, 28, 250)),
                    NoticePanel(kind),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 15.0.into(),
                            ..default()
                        },
                        NoticeText(kind),
                    ));
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(16),
                            ..default()
                        })
                        .with_children(|controls| {
                            for (retry, label) in [(true, "Retry"), (false, "Dismiss")] {
                                controls
                                    .spawn((
                                        Node {
                                            padding: UiRect::all(px(6)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.22, 0.17, 0.1)),
                                        NoticeControl { kind, retry },
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new(label),
                                            TextFont {
                                                font_size: 15.0.into(),
                                                ..default()
                                            },
                                        ));
                                    });
                            }
                        });
                });
            }
        });
}

fn handle_status_pointer(
    mut pointer: ResMut<CanvasPointer>,
    mut status: ResMut<PersistenceStatus>,
    panels: Query<(&NoticePanel, &ComputedNode, &UiGlobalTransform)>,
    controls: Query<(&NoticeControl, &ComputedNode, &UiGlobalTransform)>,
) {
    let over_panel = panels.iter().any(|(panel, node, transform)| {
        panel.0.notice(&status).visible() && pointer.is_over(node, transform)
    });
    if !over_panel {
        return;
    }
    if pointer.just_clicked() {
        for (control, node, transform) in &controls {
            if pointer.is_over(node, transform) {
                let notice = control.kind.notice_mut(&mut status);
                if !notice.visible() {
                    continue;
                }
                if control.retry && notice.failed {
                    notice.retry_requested = true;
                } else if !control.retry {
                    notice.dismissed = true;
                }
            }
        }
    }
    // Menus use the same pointer hit tests. Prevent click-through and hover-through.
    pointer.consume();
}

fn retry_options(
    store: Res<OptionsStore>,
    options: Res<PlayerOptions>,
    mut load: ResMut<OptionsLoadStatus>,
    mut status: ResMut<PersistenceStatus>,
) {
    if std::mem::take(&mut status.options.retry_requested) && status.options.failed {
        persist_player_options(&store, &options, &mut load, &mut status.options);
    }
}

fn refresh_status_ui(
    status: Res<PersistenceStatus>,
    mut panels: Query<(&NoticePanel, &mut Node), Without<NoticeControl>>,
    mut texts: Query<(&NoticeText, &mut Text)>,
    mut controls: Query<(&NoticeControl, &mut Node), Without<NoticePanel>>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if panel.0.notice(&status).visible() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (role, mut text) in &mut texts {
        text.0.clone_from(&role.0.notice(&status).message);
    }
    for (control, mut node) in &mut controls {
        node.display = if control.retry && !control.kind.notice(&status).failed {
            Display::None
        } else {
            Display::Flex
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Affine2;
    use std::{fs, time::SystemTime};

    fn temporary_store(label: &str) -> OptionsStore {
        let path = std::env::temp_dir().join(format!(
            "rpg-s1-rf05-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        OptionsStore::new(path)
    }

    fn retry_app(store: OptionsStore, options: PlayerOptions) -> App {
        let mut app = App::new();
        app.insert_resource(store)
            .insert_resource(options)
            .init_resource::<OptionsLoadStatus>()
            .init_resource::<PersistenceStatus>()
            .add_systems(Update, retry_options);
        app
    }

    fn notice_app(store: OptionsStore) -> App {
        let mut app = App::new();
        app.insert_resource(store)
            .init_resource::<PlayerOptions>()
            .init_resource::<OptionsLoadStatus>()
            .init_resource::<CanvasPointer>()
            .insert_resource(Time::<Real>::default())
            .add_plugins(PersistenceStatusPlugin);
        app
    }

    #[test]
    fn successful_notices_expire_without_input_and_new_reports_restart_the_lifetime() {
        for kind in [NoticeKind::Autosave, NoticeKind::Options] {
            let store = temporary_store("notice-expiry");
            let mut app = notice_app(store.clone());
            let failure_kind = match kind {
                NoticeKind::Autosave => NoticeKind::Options,
                NoticeKind::Options => NoticeKind::Autosave,
            };
            {
                let mut status = app.world_mut().resource_mut::<PersistenceStatus>();
                kind.notice_mut(&mut status)
                    .report("Saved to disk.".into(), false);
                failure_kind
                    .notice_mut(&mut status)
                    .report("Save failed".into(), true);
            }
            app.update();
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_secs(2));
            app.update();
            assert!(
                kind.notice(app.world().resource::<PersistenceStatus>())
                    .visible()
            );

            // An identical later success must still get a full three seconds on screen.
            kind.notice_mut(&mut app.world_mut().resource_mut::<PersistenceStatus>())
                .report("Saved to disk.".into(), false);
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_secs(2));
            app.update();
            assert!(
                kind.notice(app.world().resource::<PersistenceStatus>())
                    .visible()
            );
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_secs(1));
            app.update();
            assert!(
                !kind
                    .notice(app.world().resource::<PersistenceStatus>())
                    .visible()
            );
            let failure = failure_kind.notice(app.world().resource::<PersistenceStatus>());
            assert!(failure.visible());
            assert!(failure.failed);
            assert_eq!(failure.message, "Save failed");
            for (panel, node) in app
                .world_mut()
                .query::<(&NoticePanel, &Node)>()
                .iter(app.world())
            {
                assert_eq!(
                    node.display,
                    if panel
                        .0
                        .notice(app.world().resource::<PersistenceStatus>())
                        .failed
                    {
                        Display::Flex
                    } else {
                        Display::None
                    }
                );
            }
            fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn an_expired_success_notice_does_not_consume_pointer_input() {
        let store = temporary_store("expired-pointer");
        let mut app = notice_app(store.clone());
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .autosave
            .report("Autosave saved to disk.".into(), false);
        app.update();
        let panel = app
            .world_mut()
            .query::<(Entity, &NoticePanel)>()
            .iter(app.world())
            .find(|(_, panel)| matches!(panel.0, NoticeKind::Autosave))
            .unwrap()
            .0;
        app.world_mut().entity_mut(panel).insert((
            ComputedNode {
                size: Vec2::new(100.0, 100.0),
                ..default()
            },
            UiGlobalTransform::from(Affine2::from_translation(Vec2::new(50.0, 50.0))),
        ));
        app.insert_resource(CanvasPointer::at_viewport_position(
            Vec2::new(50.0, 50.0),
            true,
        ));
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(3));
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().display,
            Display::None
        );
        assert!(app.world().resource::<CanvasPointer>().just_clicked());
        fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
    }

    #[test]
    fn options_write_and_sync_failures_keep_live_choices_and_previous_file_until_retry() {
        for operation in ["write temporary options", "sync temporary options"] {
            let mut store = temporary_store(operation);
            store.store(&PlayerOptions::default()).unwrap();
            let previous = fs::read(store.path()).unwrap();
            store.fail_operation = Some(operation);
            let options = PlayerOptions {
                auto_enemy_turns: true,
                ..default()
            };
            let mut app = retry_app(store.clone(), options.clone());
            {
                let mut status = app.world_mut().resource_mut::<PersistenceStatus>();
                persist_player_options(
                    &store,
                    &options,
                    &mut OptionsLoadStatus::Loaded,
                    &mut status.options,
                );
                assert!(status.options.message.contains("active for this session"));
                assert!(status.options.failed);
                assert!(status.options.visible());
            }
            assert_eq!(fs::read(store.path()).unwrap(), previous);
            assert_eq!(
                fs::read_dir(store.path().parent().unwrap())
                    .unwrap()
                    .count(),
                1
            );
            app.world_mut()
                .resource_mut::<OptionsStore>()
                .fail_operation = None;
            app.update();
            app.update();
            assert_eq!(
                fs::read(store.path()).unwrap(),
                previous,
                "repair alone does not retry"
            );
            app.world_mut()
                .resource_mut::<PersistenceStatus>()
                .options
                .retry_requested = true;
            app.update();
            assert_eq!(store.load().unwrap().0, options);
            assert_eq!(*app.world().resource::<PlayerOptions>(), options);
            let notice = &app.world().resource::<PersistenceStatus>().options;
            assert!(!notice.failed);
            assert!(!notice.retry_requested);
            assert!(notice.message.contains("saved to disk"));
            fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn unwritable_options_directory_recovers_only_after_deliberate_retry() {
        let store = temporary_store("readonly");
        store.store(&PlayerOptions::default()).unwrap();
        let root = store.path().parent().unwrap().to_owned();
        let previous = fs::read(store.path()).unwrap();
        let original_permissions = fs::metadata(&root).unwrap().permissions();
        let mut readonly = original_permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&root, readonly).unwrap();
        let options = PlayerOptions {
            auto_enemy_turns: true,
            ..default()
        };
        let mut app = retry_app(store.clone(), options.clone());
        persist_player_options(
            &store,
            &options,
            &mut OptionsLoadStatus::Loaded,
            &mut app.world_mut().resource_mut::<PersistenceStatus>().options,
        );
        let failure = app
            .world()
            .resource::<PersistenceStatus>()
            .options
            .message
            .clone();
        fs::set_permissions(&root, original_permissions).unwrap();
        assert!(
            app.world().resource::<PersistenceStatus>().options.failed,
            "{failure}"
        );
        assert!(failure.contains("create temporary options"), "{failure}");
        assert_eq!(fs::read(store.path()).unwrap(), previous);
        app.update();
        assert_eq!(fs::read(store.path()).unwrap(), previous);
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .retry_requested = true;
        app.update();
        assert!(!app.world().resource::<PersistenceStatus>().options.failed);
        assert_eq!(store.load().unwrap().0, options);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn options_directory_sync_failure_reports_complete_publication_but_uncertain_durability() {
        let mut store = temporary_store("config-sync");
        store.store(&PlayerOptions::default()).unwrap();
        store.fail_operation = Some("sync config directory");
        let options = PlayerOptions {
            auto_enemy_turns: true,
            ..default()
        };
        let mut app = retry_app(store.clone(), options.clone());
        persist_player_options(
            &store,
            &options,
            &mut OptionsLoadStatus::Loaded,
            &mut app.world_mut().resource_mut::<PersistenceStatus>().options,
        );
        assert_eq!(store.load().unwrap().0, options);
        let notice = &app.world().resource::<PersistenceStatus>().options;
        assert!(notice.failed);
        assert!(notice.message.contains("file was published"));
        assert!(notice.message.contains("durability could not be confirmed"));
        app.world_mut()
            .resource_mut::<OptionsStore>()
            .fail_operation = None;
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .retry_requested = true;
        app.update();
        assert!(!app.world().resource::<PersistenceStatus>().options.failed);
        fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
    }

    #[test]
    fn retry_does_not_bypass_rejected_options_even_if_file_is_removed() {
        let store = temporary_store("rejected-retry");
        let original = b"version: 99\n";
        fs::write(store.path(), original).unwrap();
        let (options, load) = load_player_options(&store);
        let mut app = retry_app(store.clone(), options.clone());
        app.insert_resource(load.clone());
        persist_player_options(
            &store,
            &options,
            &mut load.clone(),
            &mut app.world_mut().resource_mut::<PersistenceStatus>().options,
        );
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .retry_requested = true;
        app.update();
        assert_eq!(fs::read(store.path()).unwrap(), original);
        assert_eq!(
            fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1,
            "retry creates no backup"
        );
        fs::remove_file(store.path()).unwrap();
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .retry_requested = true;
        app.update();
        assert!(!store.path().exists());
        assert!(
            app.world()
                .resource::<PersistenceStatus>()
                .options
                .message
                .contains("Reset To Defaults")
        );
        fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
    }

    #[test]
    fn pointer_retry_and_dismiss_do_not_click_through_and_failure_survives_dismissal() {
        let mut app = App::new();
        app.init_resource::<PersistenceStatus>()
            .insert_resource(CanvasPointer::at_viewport_position(
                Vec2::new(50.0, 50.0),
                true,
            ))
            .add_systems(PreUpdate, handle_status_pointer)
            .add_systems(PostUpdate, refresh_status_ui);
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .report("Options write failed".to_owned(), true);
        let transform = UiGlobalTransform::from(Affine2::from_translation(Vec2::new(50.0, 50.0)));
        let node = || ComputedNode {
            size: Vec2::new(100.0, 100.0),
            ..default()
        };
        app.world_mut().spawn((
            NoticePanel(NoticeKind::Options),
            Node::default(),
            node(),
            transform,
        ));
        let control = app
            .world_mut()
            .spawn((
                NoticeControl {
                    kind: NoticeKind::Options,
                    retry: true,
                },
                Node::default(),
                node(),
                transform,
            ))
            .id();
        let text = app
            .world_mut()
            .spawn((NoticeText(NoticeKind::Options), Text::new("")))
            .id();
        app.update();
        assert!(
            app.world()
                .resource::<PersistenceStatus>()
                .options
                .retry_requested
        );
        assert!(!app.world().resource::<CanvasPointer>().just_clicked());
        assert!(
            !app.world()
                .resource::<CanvasPointer>()
                .is_over(&node(), &transform)
        );
        assert_eq!(
            app.world().get::<Text>(text).unwrap().0,
            "Options write failed"
        );
        app.world_mut()
            .get_mut::<NoticeControl>(control)
            .unwrap()
            .retry = false;
        app.insert_resource(CanvasPointer::at_viewport_position(
            Vec2::new(50.0, 50.0),
            true,
        ));
        app.update();
        let notice = &app.world().resource::<PersistenceStatus>().options;
        assert!(!notice.visible());
        assert!(notice.failed);
        assert_eq!(notice.message, "Options write failed");
        app.world_mut()
            .resource_mut::<PersistenceStatus>()
            .options
            .report("New failure".to_owned(), true);
        assert!(
            app.world()
                .resource::<PersistenceStatus>()
                .options
                .visible()
        );
    }
}
