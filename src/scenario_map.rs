//! Source-authored map metadata schemas.
//!
//! The pinned `0897035` scenario contains 43 mapping-root files beneath `data/maps/`.
//! Metadata is found by the TMX filename stem; it does not contain a TMX path. Sixteen field-map
//! files omit `id`, in which case the filename stem is the effective identity. Signs are not
//! represented here because Python discovers them from configured tiles painted into TMX.
//!
//! NPC defaults are compatibility rules observed in `engine/world/npc_loader.py`: dialogue falls
//! back to the NPC id, facing defaults to down, presence conditions are empty, animation defaults
//! to still at speed 1.0 with wander range 2, and interaction range defaults to 1.5 tiles.

use std::num::NonZeroU32;

use serde::{Deserialize, Deserializer};

use crate::scenario_class::{PositiveFinite, UnitInterval};
use crate::scenario_condition::FlagConditions;
use crate::scenario_path::ScenarioRelativePath;
use crate::scenario_spatial::{CardinalDirection, Position};
use crate::scenario_yaml::deserialize_string;

/// One same-stem YAML metadata document beneath `data/maps/`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapMetadata {
    /// Authored identity. Missing values use the containing filename stem.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub id: Option<String>,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub warp_order: Option<u32>,
    /// Audio-index key, not a path.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub bgm: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub inn: Option<InnMetadata>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub shop: Option<ShopMetadata>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub weapon_shop: Option<ShopMetadata>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub armor_shop: Option<ShopMetadata>,
    #[serde(default)]
    pub npcs: Vec<NpcMetadata>,
    #[serde(default)]
    pub item_boxes: Vec<ItemBoxMetadata>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub enemy_spawn: Option<EnemySpawnMetadata>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub transport: Option<TransportMetadata>,
}

impl MapMetadata {
    /// Returns the authored id, or the same-stem filename identity when `id` is absent.
    pub fn effective_id<'a>(&'a self, filename_stem: &'a str) -> &'a str {
        self.id.as_deref().unwrap_or(filename_stem)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InnMetadata {
    pub cost: NonZeroU32,
    pub position: Position,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShopMetadata {
    pub items: Vec<ShopItem>,
}

/// The three closed shop-row signatures authored in the pinned map corpus.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ShopItem {
    Detailed(DetailedShopItem),
    Stocked(StockedShopItem),
    Equipment(EquipmentShopItem),
}

impl ShopItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Detailed(item) => &item.id,
            Self::Stocked(item) => &item.id,
            Self::Equipment(item) => &item.id,
        }
    }

    pub fn buy_price(&self) -> NonZeroU32 {
        match self {
            Self::Detailed(item) => item.buy_price,
            Self::Stocked(item) => item.buy_price,
            Self::Equipment(item) => item.buy_price,
        }
    }

    pub fn unlock_flag(&self) -> &str {
        match self {
            Self::Detailed(item) => &item.unlock_flag,
            Self::Stocked(item) => &item.unlock_flag,
            Self::Equipment(item) => &item.unlock_flag,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DetailedShopItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub buy_price: NonZeroU32,
    pub tags: Vec<ShopItemTag>,
    #[serde(deserialize_with = "deserialize_string")]
    pub unlock_flag: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StockedShopItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    pub buy_price: NonZeroU32,
    pub qty: NonZeroU32,
    #[serde(deserialize_with = "deserialize_string")]
    pub unlock_flag: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentShopItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    pub buy_price: NonZeroU32,
    #[serde(deserialize_with = "deserialize_string")]
    pub unlock_flag: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShopItemTag {
    Consumable,
    Recovery,
    Status,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NpcMetadata {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    /// Dialogue catalog id. Explicit source paths are not used by the pinned map corpus.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub dialogue: Option<String>,
    pub position: Position,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub sprite: Option<ScenarioRelativePath>,
    #[serde(default = "default_facing")]
    pub default_facing: CardinalDirection,
    #[serde(default)]
    pub present: FlagConditions,
    #[serde(default)]
    pub animation: NpcAnimation,
    #[serde(default = "default_interaction_range")]
    pub interaction_range: PositiveFinite,
    #[serde(
        default,
        rename = "type",
        deserialize_with = "deserialize_present_option"
    )]
    pub npc_type: Option<NpcType>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub availability: Option<UnitInterval>,
    /// Dialogue catalog id used when an unavailable guide needs an excuse.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub excuses: Option<String>,
}

impl NpcMetadata {
    /// Returns the authored dialogue id, or the NPC id when dialogue is absent.
    pub fn effective_dialogue_id(&self) -> &str {
        self.dialogue.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NpcType {
    Guide,
    Gate,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NpcAnimation {
    #[serde(default)]
    pub mode: NpcAnimationMode,
    #[serde(default = "default_animation_speed")]
    pub speed: PositiveFinite,
    #[serde(default = "default_wander_range")]
    pub range: NonZeroU32,
}

impl Default for NpcAnimation {
    fn default() -> Self {
        Self {
            mode: NpcAnimationMode::Still,
            speed: default_animation_speed(),
            range: default_wander_range(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NpcAnimationMode {
    #[default]
    Still,
    Step,
    Wander,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxMetadata {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    pub position: Position,
    #[serde(default)]
    pub present: FlagConditions,
    #[serde(default)]
    pub loot: ItemBoxLoot,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxLoot {
    #[serde(default)]
    pub items: Vec<ItemBoxLootItem>,
    #[serde(default)]
    pub magic_cores: Vec<ItemBoxMagicCore>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxLootItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(default = "one")]
    pub qty: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxMagicCore {
    pub size: MagicCoreSize,
    #[serde(default = "one")]
    pub qty: NonZeroU32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MagicCoreSize {
    Xs,
    S,
    M,
    L,
    Xl,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemySpawnMetadata {
    pub init: NonZeroU32,
    pub max: NonZeroU32,
    pub interval: PositiveFinite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransportMetadata {
    pub sail: TransportModeMetadata,
    pub fly: TransportModeMetadata,
    pub warp: TransportModeMetadata,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransportModeMetadata {
    #[serde(deserialize_with = "deserialize_string")]
    pub unlock_flag: String,
    pub origin: TransportOrigin,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub destinations: Option<TransportDestinations>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransportOrigin {
    PortTile,
    WorldMapAny,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransportDestinations {
    VisitedOnly,
}

fn default_facing() -> CardinalDirection {
    CardinalDirection::Down
}

fn default_animation_speed() -> PositiveFinite {
    PositiveFinite::new(1.0).expect("1.0 is positive and finite")
}

fn default_wander_range() -> NonZeroU32 {
    NonZeroU32::new(2).expect("two is non-zero")
}

fn default_interaction_range() -> PositiveFinite {
    PositiveFinite::new(1.5).expect("1.5 is positive and finite")
}

fn one() -> NonZeroU32 {
    NonZeroU32::new(1).expect("one is non-zero")
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string(deserializer).map(Some)
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroU32;
    use std::path::Path;

    use super::{
        EquipmentShopItem, MagicCoreSize, MapMetadata, NpcAnimationMode, NpcType, ShopItem,
        ShopItemTag, StockedShopItem, TransportDestinations, TransportOrigin,
    };
    use crate::scenario_spatial::{CardinalDirection, Position};
    use crate::scenario_yaml;

    #[test]
    fn loads_every_ardel_map_metadata_field_and_shop_variant() {
        let map: MapMetadata = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/ardel-map-metadata-complete.yaml"
        ))
        .expect("the source-shaped Ardel fixture should deserialize");

        assert_eq!(map.effective_id("ignored_stem"), "town_01_ardel");
        assert_eq!(map.name, "Ardel Village");
        assert_eq!(map.warp_order, Some(10));
        assert_eq!(map.bgm.as_deref(), Some("town.default"));

        let inn = map.inn.as_ref().expect("Ardel has an inn");
        assert_eq!(inn.cost, NonZeroU32::new(50).unwrap());
        assert_eq!(inn.position, Position::new(8, 5));

        let shop = map.shop.as_ref().expect("Ardel has an item shop");
        assert_eq!(shop.items.len(), 2);
        let ShopItem::Detailed(potion) = &shop.items[0] else {
            panic!("Ardel item-shop rows use the detailed signature");
        };
        assert_eq!(potion.id, "potion");
        assert_eq!(potion.name, "Potion");
        assert_eq!(potion.buy_price, NonZeroU32::new(100).unwrap());
        assert_eq!(
            potion.tags,
            [ShopItemTag::Consumable, ShopItemTag::Recovery]
        );
        assert_eq!(potion.unlock_flag, "story_quest_started");

        assert!(matches!(
            &map.weapon_shop.as_ref().unwrap().items[0],
            ShopItem::Equipment(EquipmentShopItem { id, .. }) if id == "wooden_stick"
        ));
        assert!(matches!(
            &map.armor_shop.as_ref().unwrap().items[0],
            ShopItem::Equipment(EquipmentShopItem { id, .. }) if id == "cloth_hood"
        ));

        assert_eq!(map.npcs.len(), 5);
        let elise = &map.npcs[0];
        assert_eq!(elise.dialogue.as_deref(), Some("elise_join"));
        assert_eq!(elise.position, Position::new(12, 6));
        assert_eq!(
            elise.sprite.as_ref().unwrap().as_str(),
            "assets/sprites/party/02_elise_walk.tsx"
        );
        assert_eq!(elise.default_facing, CardinalDirection::Down);
        assert_eq!(elise.present.excludes, ["npc_elise_joined"]);
        assert_eq!(elise.animation.mode, NpcAnimationMode::Step);
        assert_eq!(elise.animation.speed.get(), 0.7);
        assert_eq!(elise.animation.range, NonZeroU32::new(2).unwrap());

        let guide = &map.npcs[1];
        assert_eq!(guide.npc_type, Some(NpcType::Guide));
        assert_eq!(guide.availability.unwrap().get(), 0.30);
        assert_eq!(guide.excuses.as_deref(), Some("guide_excuses"));
        assert_eq!(guide.animation.mode, NpcAnimationMode::Wander);
        assert_eq!(guide.animation.speed.get(), 1.8);
        assert_eq!(guide.animation.range, NonZeroU32::new(5).unwrap());

        let shopkeeper = &map.npcs[2];
        assert_eq!(shopkeeper.interaction_range.get(), 2.5);
        assert_eq!(map.npcs[3].default_facing, CardinalDirection::Left);
        assert_eq!(map.npcs[4].default_facing, CardinalDirection::Right);
    }

    #[test]
    fn applies_only_observed_map_and_npc_defaults() {
        let map: MapMetadata = scenario_yaml::from_str(
            "name: Unnamed Field\nnpcs:\n  - id: watcher\n    name: Watcher\n    position: [1, 2]\n",
        )
        .expect("observed optional fields may be omitted");

        assert_eq!(map.effective_id("zone_99_test"), "zone_99_test");
        assert!(map.bgm.is_none());
        assert!(map.shop.is_none());
        assert!(map.item_boxes.is_empty());

        let npc = &map.npcs[0];
        assert_eq!(npc.effective_dialogue_id(), "watcher");
        assert!(npc.sprite.is_none());
        assert_eq!(npc.default_facing, CardinalDirection::Down);
        assert!(npc.present.requires.is_empty());
        assert!(npc.present.excludes.is_empty());
        assert_eq!(npc.animation.mode, NpcAnimationMode::Still);
        assert_eq!(npc.animation.speed.get(), 1.0);
        assert_eq!(npc.animation.range, NonZeroU32::new(2).unwrap());
        assert_eq!(npc.interaction_range.get(), 1.5);
    }

    #[test]
    fn loads_stocked_shop_boxes_spawn_and_transport_corpus_variants() {
        let map: MapMetadata = scenario_yaml::from_str(
            r#"name: Greenwood Forest
shop:
  items:
    - id: potion
      buy_price: 100
      qty: 5
      unlock_flag: story_quest_started
item_boxes:
  - id: forest_chest_01
    position: [47, 30]
    loot:
      items: [{ id: potion }]
      magic_cores: [{ size: m, qty: 3 }]
enemy_spawn: { init: 3, max: 6, interval: 25.0 }
transport:
  sail: { unlock_flag: transport_sail_unlocked, origin: port_tile }
  fly: { unlock_flag: transport_fly_unlocked, origin: world_map_any }
  warp: { unlock_flag: transport_warp_unlocked, origin: world_map_any, destinations: visited_only }
"#,
        )
        .expect("the remaining pinned map-metadata variants should deserialize");

        assert!(matches!(
            &map.shop.unwrap().items[0],
            ShopItem::Stocked(StockedShopItem { qty, .. }) if *qty == NonZeroU32::new(5).unwrap()
        ));
        assert_eq!(map.item_boxes[0].loot.items[0].qty.get(), 1);
        assert_eq!(map.item_boxes[0].loot.magic_cores[0].size, MagicCoreSize::M);
        assert_eq!(map.enemy_spawn.unwrap().interval.get(), 25.0);
        let transport = map.transport.unwrap();
        assert_eq!(transport.sail.origin, TransportOrigin::PortTile);
        assert_eq!(transport.fly.origin, TransportOrigin::WorldMapAny);
        assert_eq!(
            transport.warp.destinations,
            Some(TransportDestinations::VisitedOnly)
        );
    }

    #[test]
    fn rejects_unknown_fields_invalid_variants_and_unobserved_nulls() {
        for document in [
            "name: Ardel\nbackground_music: town.default\n",
            "name: Ardel\nbgm: null\n",
            "name: Ardel\ninn: { cost: 50 }\n",
            "name: Ardel\nshop: { items: [{ id: potion, buy_price: 100, unlock_flag: ready, typo: true }] }\n",
            "name: Ardel\nnpcs: [{ id: ellen, name: Ellen, position: [1, 2], default_facing: north }]\n",
            "name: Ardel\nnpcs: [{ id: ellen, name: Ellen, position: [1, 2], animation: { mode: dance } }]\n",
            "name: Ardel\nnpcs: [{ id: ellen, name: Ellen, position: [1, 2], animation: { speed: 0.0 } }]\n",
            "name: Ardel\nnpcs: [{ id: ellen, name: Ellen, position: [1, 2], sprite: ../outside.tsx }]\n",
        ] {
            assert!(
                scenario_yaml::from_str::<MapMetadata>(document).is_err(),
                "unexpectedly accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_map_metadata_file_when_source_is_available() {
        let maps = std::env::var_os("RPG_S1_PINNED_MAPS_DIR")
            .expect("RPG_S1_PINNED_MAPS_DIR must name the pinned data/maps directory");
        let mut files = fs::read_dir(Path::new(&maps))
            .expect("map metadata directory should be readable")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let mut missing_ids = 0;
        for path in &files {
            let document = fs::read_to_string(path).expect("map YAML should be readable");
            let map: MapMetadata = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            missing_ids += usize::from(map.id.is_none());
        }

        assert_eq!(files.len(), 43);
        assert_eq!(missing_ids, 16);
    }
}
