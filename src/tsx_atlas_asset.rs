//! Bevy-native loading and atlas projection for one external TSX atlas.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.10 establishes atlas assets before M4.11 and later world rendering consume them"
    )
)]

use std::{error::Error, fmt, path::PathBuf, str};

use bevy::{
    asset::{Asset, AssetApp, AssetLoader, AssetPath, Handle, LoadContext, io::Reader},
    image::{Image, ImageLoaderSettings, ImageSampler, TextureAtlas, TextureAtlasLayout},
    math::{URect, UVec2},
    prelude::{App, FromWorld, Plugin, Sprite, World},
    reflect::TypePath,
};

use crate::{
    scenario_path::{ScenarioRelativePath, ScenarioRelativePathError},
    scenario_root::ScenarioRoot,
    tsx_metadata::{TsxMetadataError, TsxTilesetMetadata, parse_tsx_tileset_metadata},
};

const ATLAS_LAYOUT_LABEL: &str = "atlas_layout";

/// Registers the external TSX asset and its project-owned loader.
pub(crate) struct TsxAtlasAssetPlugin;

impl Plugin for TsxAtlasAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioRoot>()
            .init_asset::<TextureAtlasLayout>()
            .init_asset::<TsxAtlasAsset>()
            .init_asset_loader::<TsxAtlasAssetLoader>();
    }
}

/// A parsed TSX atlas whose image and atlas layout are both Bevy dependencies.
#[derive(Asset, Debug, TypePath)]
pub(crate) struct TsxAtlasAsset {
    metadata: TsxTilesetMetadata,
    #[dependency]
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl TsxAtlasAsset {
    pub(crate) const fn metadata(&self) -> &TsxTilesetMetadata {
        &self.metadata
    }

    pub(crate) const fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub(crate) const fn layout(&self) -> &Handle<TextureAtlasLayout> {
        &self.layout
    }

    /// Creates the exact Bevy sprite representation for one tileset-local tile.
    pub(crate) fn sprite_for_tile(&self, tile_id: u32) -> Result<Sprite, TsxAtlasTileError> {
        if tile_id >= self.metadata.tile_count() {
            return Err(TsxAtlasTileError {
                tile_id,
                tile_count: self.metadata.tile_count(),
            });
        }

        Ok(Sprite::from_atlas_image(
            self.image.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index: tile_id as usize,
            },
        ))
    }
}

/// A requested local tile is not part of the TSX atlas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TsxAtlasTileError {
    tile_id: u32,
    tile_count: u32,
}

impl fmt::Display for TsxAtlasTileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "tile id {} is outside atlas tilecount {}",
            self.tile_id, self.tile_count
        )
    }
}

impl Error for TsxAtlasTileError {}

#[derive(TypePath)]
struct TsxAtlasAssetLoader {
    scenario_root: ScenarioRoot,
}

impl FromWorld for TsxAtlasAssetLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            scenario_root: world.resource::<ScenarioRoot>().clone(),
        }
    }
}

impl AssetLoader for TsxAtlasAssetLoader {
    type Asset = TsxAtlasAsset;
    type Settings = ();
    type Error = TsxAtlasAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let owner = scenario_owner(load_context.path(), &self.scenario_root)?;
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(TsxAtlasAssetLoaderError::Io)?;
        let document = str::from_utf8(&bytes).map_err(TsxAtlasAssetLoaderError::Utf8)?;
        let metadata = parse_tsx_tileset_metadata(document, &owner)
            .map_err(TsxAtlasAssetLoaderError::Metadata)?;

        let image_path = AssetPath::from_path_buf(PathBuf::from(
            self.scenario_root.resolve(metadata.image().source()),
        ))
        .with_source(load_context.path().source().clone_owned());
        // Linear filtering blends adjacent atlas cells at subpixel positions, exposing tile
        // boundaries as a flickering grid while characters or the camera are moving.
        let image = load_context
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load(image_path);
        let layout = load_context.add_labeled_asset(ATLAS_LAYOUT_LABEL, atlas_layout(&metadata));

        Ok(TsxAtlasAsset {
            metadata,
            image,
            layout,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tsx"]
    }
}

fn scenario_owner(
    asset_path: &AssetPath<'_>,
    scenario_root: &ScenarioRoot,
) -> Result<ScenarioRelativePath, TsxAtlasAssetLoaderError> {
    let relative = asset_path
        .path()
        .strip_prefix(scenario_root.logical_root())
        .map_err(|_| TsxAtlasAssetLoaderError::OutsideScenarioRoot)?;
    let relative = relative
        .to_str()
        .ok_or(TsxAtlasAssetLoaderError::NonUtf8Owner)?;
    ScenarioRelativePath::try_from(relative).map_err(TsxAtlasAssetLoaderError::OwnerPath)
}

fn atlas_layout(metadata: &TsxTilesetMetadata) -> TextureAtlasLayout {
    let tile_size = UVec2::new(metadata.tile_width(), metadata.tile_height());
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(
        metadata.image().width(),
        metadata.image().height(),
    ));

    for tile_id in 0..metadata.tile_count() {
        let cell = UVec2::new(tile_id % metadata.columns(), tile_id / metadata.columns());
        let min = cell * tile_size;
        layout.add_texture(URect {
            min,
            max: min + tile_size,
        });
    }
    layout
}

#[derive(Debug)]
enum TsxAtlasAssetLoaderError {
    OutsideScenarioRoot,
    NonUtf8Owner,
    OwnerPath(ScenarioRelativePathError),
    Io(std::io::Error),
    Utf8(str::Utf8Error),
    Metadata(TsxMetadataError),
}

impl fmt::Display for TsxAtlasAssetLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideScenarioRoot => {
                formatter.write_str("TSX asset is outside the active scenario root")
            }
            Self::NonUtf8Owner => formatter.write_str("TSX asset path must be UTF-8"),
            Self::OwnerPath(error) => write!(formatter, "invalid TSX owner path: {error}"),
            Self::Io(error) => write!(formatter, "TSX read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "TSX is not UTF-8: {error}"),
            Self::Metadata(error) => write!(formatter, "TSX metadata is invalid: {error}"),
        }
    }
}

impl Error for TsxAtlasAssetLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OwnerPath(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Metadata(error) => Some(error),
            Self::OutsideScenarioRoot | Self::NonUtf8Owner => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        asset::{AssetApp, AssetMetaCheck, AssetPlugin, AssetServer, Assets, LoadState},
        image::{CompressedImageFormats, ImageLoader, ImagePlugin, TextureAtlasLayout},
        prelude::{App, Image, MinimalPlugins, Sprite},
    };

    use super::*;

    const ASSET_BASE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/tsx-atlas-asset"
    );
    const TSX_PATH: &str = "scenarios/invented_atlas/assets/tiles/invented.tsx";
    const IMAGE_PATH: &str = "scenarios/invented_atlas/assets/tiles/invented.png";

    fn atlas_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin {
                file_path: ASSET_BASE.to_owned(),
                meta_check: AssetMetaCheck::Never,
                ..Default::default()
            })
            .add_plugins(ImagePlugin::default_linear())
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
            .insert_resource(ScenarioRoot::try_for_package_key("invented_atlas").unwrap())
            .add_plugins(TsxAtlasAssetPlugin);
        app
    }

    fn load_atlas(app: &mut App) -> Handle<TsxAtlasAsset> {
        let handle = app.world().resource::<AssetServer>().load(TSX_PATH);
        for _ in 0..1_000 {
            app.update();
            if app
                .world()
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(handle.id())
            {
                return handle;
            }
            if let LoadState::Failed(error) = app
                .world()
                .resource::<AssetServer>()
                .load_state(handle.id())
            {
                panic!("invented TSX load failed: {error}");
            }
            std::thread::yield_now();
        }
        let server = app.world().resource::<AssetServer>();
        let atlas = app
            .world()
            .resource::<Assets<TsxAtlasAsset>>()
            .get(&handle)
            .expect("loaded root should be present while diagnosing dependencies");
        panic!(
            "invented TSX and its PNG dependency did not finish loading: root={:?}, recursive={:?}, image={:?}, layout={:?}",
            server.load_state(handle.id()),
            server.recursive_dependency_load_state(handle.id()),
            server.load_state(atlas.image().id()),
            server.load_state(atlas.layout().id())
        );
    }

    #[test]
    fn external_tsx_loads_png_and_spawns_the_expected_atlas_tile_sprite() {
        let mut app = atlas_app();
        let atlas_handle = load_atlas(&mut app);

        let atlas = app
            .world()
            .resource::<Assets<TsxAtlasAsset>>()
            .get(&atlas_handle)
            .expect("loaded TSX asset should be published");
        assert_eq!(atlas.metadata().tile_count(), 6);
        assert_eq!(
            atlas.image().path().unwrap().path().to_string_lossy(),
            IMAGE_PATH
        );
        let loaded_image = app
            .world()
            .resource::<Assets<Image>>()
            .get(atlas.image())
            .expect("PNG dependency should be decoded before recursive load is ready");
        assert_eq!(loaded_image.size(), UVec2::new(6, 4));
        assert_eq!(loaded_image.sampler, ImageSampler::nearest());

        let expected_image = atlas.image().clone();
        let expected_layout = atlas.layout().clone();
        let sprite = atlas.sprite_for_tile(4).unwrap();
        let entity = app.world_mut().spawn(sprite).id();
        let sprite = app.world().entity(entity).get::<Sprite>().unwrap();
        assert_eq!(sprite.image, expected_image);
        let texture_atlas = sprite
            .texture_atlas
            .as_ref()
            .expect("TSX tile sprite must select an atlas section");
        assert_eq!(texture_atlas.layout, expected_layout);
        assert_eq!(texture_atlas.index, 4);

        let layouts = app.world().resource::<Assets<TextureAtlasLayout>>();
        let layout = layouts.get(&texture_atlas.layout).unwrap();
        assert_eq!(layout.size, UVec2::new(6, 4));
        assert_eq!(layout.textures.len(), 6);
        assert_eq!(
            texture_atlas.texture_rect(layouts),
            Some(URect {
                min: UVec2::new(2, 2),
                max: UVec2::new(4, 4),
            })
        );
    }

    #[test]
    fn sprite_constructor_rejects_tilecount_boundary() {
        let mut app = atlas_app();
        let atlas_handle = load_atlas(&mut app);
        let atlas = app
            .world()
            .resource::<Assets<TsxAtlasAsset>>()
            .get(&atlas_handle)
            .unwrap();

        assert_eq!(
            atlas.sprite_for_tile(6).unwrap_err().to_string(),
            "tile id 6 is outside atlas tilecount 6"
        );
    }
}
