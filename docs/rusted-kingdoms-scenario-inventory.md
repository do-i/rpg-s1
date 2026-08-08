# Rusted Kingdoms Scenario Data Inventory

Snapshot: `../agentic-rpg` at `08970359d6cb03586948625d29b0d3351dbbf785`

This is an observed inventory of the pinned Python scenario, not a decision
about the Rust runtime's data format or asset layout. Counts are regular files
unless noted otherwise.

## Reproduce

From the target repository root, use the following rules:

```bash
scenario=../agentic-rpg/rusted_kingdoms
find "$scenario/data/maps" -maxdepth 1 -type f -name '*.yaml' | wc -l
find "$scenario/assets/maps" -maxdepth 1 -type f -name '*.tmx' | wc -l
find "$scenario/assets/maps" -maxdepth 1 -type f -name '*.tmx.bak' | wc -l
find "$scenario/assets" -type f | wc -l
find "$scenario/assets" -type f -printf '%P\n' | awk -F/ '{print $1}' | sort | uniq -c
```

Record counts below were obtained by parsing YAML with `yaml.safe_load_all`.
For a list-root YAML file, a record is one list member; for a multi-document
YAML file, a record is one document. Audio entries are scalar leaves in their
respective index trees. TMX campaign membership is the portal graph reachable
from the manifest's starting map, with the two `sample_*.tmx` files excluded.

## Scenario manifest and supporting singleton data

| Data | Path | Files | Records | Root/schema notes |
| --- | --- | ---: | ---: | --- |
| Manifest | `manifest.yaml` | 1 | 1 | Mapping: identity, title/font/UI, service sprites, protagonist/start, flags, and `refs`. |
| Party | `data/party.yaml` | 1 | 5 | Mapping with `party` list; members contain class, join/recruit, stats, equipment, and abilities. |
| Balance | `data/balance.yaml` | 1 | 1 | Mapping: progression, economy, battle, spawner, and movement. |
| Battle backgrounds | `data/battle_backgrounds.yaml` | 1 | 13 | List of background IDs and `ground_rect` mappings. |

`manifest.yaml` declares the scenario roots: `data/party.yaml`,
`data/classes/`, `data/maps/`, `data/dialogue/`, `data/items/`,
`data/enemies/`, `data/encount/`, `data/recipe/`, `data/quests.yaml`,
`data/balance.yaml`, `data/battle_backgrounds.yaml`, `assets/`, and
`assets/maps/`. Its start is map `town_01_ardel` at `[14, 5]`, with
`data/dialogue/intro_cutscene.yaml`.

## Campaign data

| Data | Root | Files | Records | Representative schema/root |
| --- | --- | ---: | ---: | --- |
| Map metadata | `data/maps/` | 43 | 43 | Mapping; town files typically have `id`, `name`, `warp_order`, `bgm`, services, and `npcs`; zone files use `name`, `warp_order`, optional `enemy_spawn`, `transport`, and `item_boxes`. Example: `data/maps/town_01_ardel.yaml`. |
| TMX maps | `assets/maps/` | 47 normal | 45 campaign, 2 samples | Tiled TMX; portals use object properties such as `target_map`, `target_position_x`, and `target_position_y`. Example: `assets/maps/town_01_ardel.tmx`. |
| TMX backups | `assets/maps/` | 2 | 0 | `town_01_ardel.tmx.bak` and `zone_02_open_plains_cave_01.tmx.bak`; not normal TMX inputs. |
| TMX samples | `assets/maps/` | 2 | 0 | `sample_01.tmx` and `sample_dungeon_01.tmx`; excluded from the portal-reachable campaign graph. |
| Dialogue | `data/dialogue/` | 91 | 91 | Mapping normally rooted at `id`, `type`, `lines`, and optional completion effects. Example: `data/dialogue/intro_cutscene.yaml`. |
| Encounter zones | `data/encount/` | 16 | 16 | Mapping: `id`, `name`, density/spawn settings, battle background, formations in `entries`, and optional `boss`. Example: `data/encount/zone_01_starting_forest.yaml`. |
| Enemy rank catalogs | `data/enemies/` | 8 | 106 | YAML multi-document stream; each enemy has identity, rank, stats, drops, AI, and targeting. Example: `data/enemies/enemies_rank_1_SS.yaml`. |
| Boss move sets | `data/enemies/boss_move_sets/` | 9 | 9 | Mapping with `ai` and `targeting`; referenced by enemy `ai_ref` rather than entity IDs. Example: `data/enemies/boss_move_sets/skeleton_knight_base.yaml`. |
| Item catalogs | `data/items/` | 13 | 185 | List-root files; item mappings include `id`, type/slot, stats, prices, and description. Example: `data/items/weapons.yaml`. |
| Classes | `data/classes/` | 5 | 5 | Mapping rooted at `class`; contains base stats, growth, equipment slots, and abilities. Example: `data/classes/hero.yaml`. |
| Recipes | `data/recipe/` | 1 | 11 | List root; each recipe has inputs, output, cost, and an unlock flag. Example: `data/recipe/all_recipe.yaml`. |
| Quests | `data/quests.yaml` | 1 | 16 | List root; each quest has identity, type, location, description, start and completion flags. |

## Audio indices

| Index | Path | Entries | Categories | Target root |
| --- | --- | ---: | --- | --- |
| BGM | `data/audio/bgm_index.yaml` | 12 | title (1), battle (2), town (2), zone (3), overworld (4) | `assets/audio/bgm/` |
| SFX | `data/audio/sfx_index.yaml` | 23 | battle (18), UI (5) | `assets/audio/sfx/` |

All 35 indexed relative audio paths exist beneath `assets/audio/` in the
pinned source. The index paths deliberately omit the `assets/audio/` prefix.

## Asset inventory

`assets/` has **807** regular files.

| Category | Files | Type breakdown |
| --- | ---: | --- |
| `audio/` | 125 | 123 `.mp3`, 1 extensionless license file, 1 `.md` README |
| `fonts/` | 4 | 2 `.ttf`, 2 license `.txt` files |
| `images/` | 31 | 26 `.webp`, 5 `.png` |
| `maps/` | 50 | 47 `.tmx`, 2 `.bak`, 1 `.tiled-project` |
| `sprites/` | 530 | 265 `.png`, 265 `.tsx` |
| `tilesets/` | 67 | 19 `.png`, 18 `.tsx`, 26 `.stamp`, 3 `.txt`, 1 `.md` |

## Observed anomalies for later tasks

- The runtime constructs map paths from the current ID as
  `assets/maps/<id>.tmx` and reads optional map metadata from
  `data/maps/<id>.yaml`; it does not use explicit TMX references in YAML.
- Of 43 map YAML stems and 47 normal TMX stems, 42 match. The unmatched map
  metadata is `data/maps/zone_05_mountain_foothills.yaml`, while the campaign
  portals use `assets/maps/zone_05_mountain_foothills_01.tmx`. The `_01` TMX
  therefore has no same-stem metadata; later schema/validation work must
  account for this source-state mismatch.
- Three campaign cave/segment TMX files lack same-stem metadata
  (`zone_02_open_plains_cave_01`, `zone_02_open_plains_cave_02`, and the
  `_01` foothills file). The Python loader treats missing map YAML as optional.
- Sixteen zone map metadata files have no `id`, using their filename stem as
  the effective identity. Two dialogue files lack `id`
  (`data/dialogue/guide_excuses.yaml` and
  `data/dialogue/port_master_intro.yaml`). Boss move-set files also lack `id`.
- `data/enemies/enemies_rank_*.yaml` is a multi-document YAML stream, unlike
  list-root item/recipe/quest files. The manifest names the encounter root
  `data/encount/` (without an `er`), which is a source spelling to preserve or
  explicitly handle in later compatibility work.
