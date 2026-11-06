//! Optional ability artwork shared by field and battle menus.

use std::collections::BTreeMap;

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use crate::{scenario_inventory::ScenarioInventory, scenario_root::ScenarioRoot};

#[derive(Component, PartialEq, Eq)]
pub(crate) struct SpellIcon(String);

impl SpellIcon {
    pub(crate) fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

#[derive(Component)]
pub(crate) struct SpellIconFallback(Handle<Image>);

pub(crate) struct SpellImagesPlugin;
impl Plugin for SpellImagesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            resolve_spell_icons.before(bevy::ui::UiSystems::Prepare),
        );
    }
}

pub(crate) fn spawn_spell_image(parent: &mut ChildSpawnerCommands<'_>, id: &str, size: f32) {
    spawn_spell_image_with_fallback(parent, id, size, None);
}

pub(crate) fn spawn_spell_image_with_fallback(
    parent: &mut ChildSpawnerCommands<'_>,
    id: &str,
    size: f32,
    fallback: Option<&Handle<Image>>,
) {
    let mut image = parent.spawn((
        SpellIcon(id.to_owned()),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Name::new(format!("Spell image: {id}")),
    ));
    if let Some(fallback) = fallback {
        image.insert(SpellIconFallback(fallback.clone()));
    }
    image.with_children(|image| {
        image.spawn((
            Node {
                width: px(size * 0.35),
                height: px(size * 0.35),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BorderColor::all(crate::menu_chrome::status_muted()),
            Name::new("Spell artwork placeholder"),
        ));
    });
}

pub(crate) fn resolve_spell_icons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    icons: Query<(Entity, &SpellIcon, Option<&SpellIconFallback>), Changed<SpellIcon>>,
    mut handles: Local<BTreeMap<String, Handle<Image>>>,
) {
    for (entity, icon, fallback) in &icons {
        let Some(path) = inventory.spell_images.get(&icon.0) else {
            if let Some(fallback) = fallback {
                commands
                    .entity(entity)
                    .despawn_children()
                    .insert(ImageNode::new(fallback.0.clone()));
            } else {
                commands.entity(entity).remove::<ImageNode>();
            }
            continue;
        };
        let path = root.resolve(path);
        let handle = handles
            .entry(path.clone())
            .or_insert_with(|| asset_server.load(path));
        commands
            .entity(entity)
            .despawn_children()
            .insert(ImageNode::new(handle.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_spell_art_uses_the_provided_fallback_image() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::image::ImagePlugin::default(),
        ))
        .insert_resource(ScenarioRoot::default())
        .insert_resource(ScenarioInventory::default())
        .add_systems(Update, resolve_spell_icons);
        let fallback = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let mut queue = bevy::ecs::world::CommandQueue::default();
        Commands::new(&mut queue, app.world())
            .spawn(Node::default())
            .with_children(|parent| {
                spawn_spell_image_with_fallback(parent, "unknown", 40.0, Some(&fallback));
            });
        queue.apply(app.world_mut());
        app.update();
        let world = app.world_mut();
        let (entity, image) = world
            .query_filtered::<(Entity, &ImageNode), With<SpellIcon>>()
            .single(world)
            .unwrap();
        assert_eq!(image.image, fallback);
        assert!(
            world
                .get::<Children>(entity)
                .is_none_or(|children| children.is_empty())
        );
    }
}
