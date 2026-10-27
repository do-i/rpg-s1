//! Shared item artwork for inventory, service, and reward UI.

use std::collections::BTreeMap;

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*, ui::UiSystems};

use crate::{scenario_inventory::ScenarioInventory, scenario_root::ScenarioRoot};

pub(crate) struct ItemImagesPlugin;

/// Native size of the scenario's item artwork in UI pixels.
pub(crate) const ITEM_IMAGE_SIZE: f32 = 128.0;

impl Plugin for ItemImagesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, resolve_item_icons.before(UiSystems::Prepare));
    }
}

/// The item identity stays with the UI node, so renderers need no asset-loading parameters.
#[derive(Component)]
pub(crate) struct ItemIcon(String);

impl ItemIcon {
    pub(crate) fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

pub(crate) fn spawn_item_image(parent: &mut ChildSpawnerCommands<'_>, id: &str) {
    parent.spawn((
        ItemIcon::new(id),
        Node {
            width: px(ITEM_IMAGE_SIZE),
            height: px(ITEM_IMAGE_SIZE),
            flex_shrink: 0.0,
            ..default()
        },
        Name::new(format!("Item image: {id}")),
    ));
}

fn resolve_item_icons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    icons: Query<(Entity, &ItemIcon), Changed<ItemIcon>>,
    mut handles: Local<BTreeMap<String, Handle<Image>>>,
) {
    for (entity, icon) in &icons {
        // Optional artwork: missing images keep the node's existing fallback and its text.
        let Some(path) = inventory.item_images.get(&icon.0) else {
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
    use std::{
        path::Path,
        thread,
        time::{Duration, Instant},
    };

    use bevy::{
        asset::AssetPlugin,
        image::{CompressedImageFormats, ImageLoader},
    };

    use super::*;

    #[test]
    fn item_icons_load_real_art_and_preserve_fallbacks_for_missing_images() {
        let asset_base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let root = ScenarioRoot::default();
        let inventory = ScenarioInventory::discover(&asset_base, &root);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin {
                file_path: asset_base.to_string_lossy().into_owned(),
                ..default()
            })
            .init_asset::<Image>()
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
            .insert_resource(root)
            .insert_resource(inventory)
            .add_plugins(ItemImagesPlugin);
        let potion = app
            .world_mut()
            .spawn((ItemIcon::new("potion"), Node::default()))
            .with_children(|node| {
                node.spawn(Text::new("USE"));
            })
            .id();
        let duplicate = app
            .world_mut()
            .spawn((ItemIcon::new("potion"), Node::default()))
            .id();
        let missing = app
            .world_mut()
            .spawn((ItemIcon::new("an_item_without_art"), Node::default()))
            .with_children(|node| {
                node.spawn(Text::new("KEY"));
            })
            .id();

        app.update();

        assert!(app.world().get::<Children>(potion).is_none());
        assert!(app.world().get::<ImageNode>(missing).is_none());
        assert!(app.world().get::<Children>(missing).is_some());
        let handle = app.world().get::<ImageNode>(potion).unwrap().image.clone();
        assert_eq!(
            handle,
            app.world().get::<ImageNode>(duplicate).unwrap().image
        );
        assert_eq!(
            app.world()
                .resource::<AssetServer>()
                .get_path(handle.id())
                .unwrap()
                .to_string(),
            "scenarios/rusted_kingdoms/media/images/items/potion.png"
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update();
            if let Some(image) = app.world().resource::<Assets<Image>>().get(&handle) {
                assert_eq!(image.size(), UVec2::new(128, 128));
                break;
            }
            assert!(Instant::now() < deadline, "the item image did not decode");
            thread::sleep(Duration::from_millis(5));
        }
    }
}
