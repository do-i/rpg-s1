# Rusted Kingdoms tile style audit

Snapshot: **2026-10-26**, current worktree, based on commit `6c16cedfcdcb44cefc9cc04c20ea54f8fafacd47` plus the current Frostholm/Ashenveil additions and other existing edits. This is the factual inventory and art-direction reference for the [local redesign plan](../tile-redesign-plan.md). GitHub publication is not required for this project.

All **88 playable maps** were inspected for authored tile usage. The audit inventories **4,723 atlas cells and standalone stair assets**. Of these, **470 nonempty cells/assets** belong to the 31 remaining redesign or harmonization families: 448 redesign, 22 harmonization. Those families account for **14,199 authored tile placements**. Every nonempty cell has an explicit disposition in the saved CSV. Empty padding is recorded separately. The implemented interior wall kit is retained as a reference.

## Scope and evidence

The scope is scenario map tiles: every TSX atlas in `media/tilesets`, external TSX files referenced by playable maps (including the closed/open item-box sprite), and both standalone stair PNG variants. Party/NPC/enemy sprites, battle backgrounds and interface art are outside map-tile scope. `terrain-v7.png` and `terrain-v7.tsx` are explicitly excluded from redesign and remain visual context references. The red collision tile is editor/runtime metadata, not game artwork.

- [Per-tile usage and dispositions](usage.csv): zero-based local IDs, descriptions, pixel presence, placements, distinct-map counts, family and exact map membership.
- [Source hashes and map list](sources.json): 250 inputs, including all playable maps/YAML, atlas PNG/TSX files, editor stamps, template, sample and the audit/classification sources.
- [Legacy atlas comparison](legacy-atlases.png) and [recent reference comparison](reference-atlases.png).
- [Ardel exterior before](ardel-before.png) and [Ardel shop before](shop-before.png): contextual Tiled renders from this snapshot. These are scaled previews; inspect original atlases at 1x for pixel-level judgments.
- Classification scopes and reasons: [families.json](../../temp/art/011-tile-style-audit/families.json).

The engine selects the adjacent open chest frame at runtime (`crates/rpg-engine/src/world_object.rs`). Its zero authored placements do not mean the open state is unused. Both frames belong to the same family. The unreferenced standalone stairs and ritual seals are recorded without inventing new map usage.

## Findings

The main remaining mismatch is between the older flat, strongly outlined, brightly colored modular art and the material depth of the newer ruins/alpine objects. The interior plaster/trim replacement was committed in `51b636f` and its applications were retiled in `00084d5`; retain it as an implemented reference. Wooden floors, town facades, merchant counters, broadleaf trees and windows now lead the remaining usage-ranked work. Gray slab counter surfaces and saturated blue/red door panels particularly weaken the medieval setting. Roof strips, furniture and small props have the same issue at smaller scales.

Recent additions are assessed visually rather than by date alone. The repaired cave rock structure and broken rope cap still inherit their older source materials, so they remain coupled to those redesign families. Newer merchant display cabinets need a smaller harmonization pass. Retained reference art is listed below; retaining it does not remove the requirement to compare new work against it in context.

| Retained family | Decision |
| --- | --- |
| Implemented interior plaster/timber walls | Retain wall IDs 0–11, 18–26 and 36–39 from commit `51b636f`, currently used 6,174 times across 51 maps. |
| Ancient Ruins gate, props, borders, courtyard and sanctum | Use their chipped stone, subdued moss, aged wood and whole-object depth as core outdoor references. |
| Alpine foothills and mountain-pass stone bridges/gateway | Use their granite/timber, snow accumulation and clear projection as regional references. The broken rope cap is included with the old rope bridge family because its seam is coupled. |
| Shrine altar, pews, votives and other props | Retain narrative identity and improved furniture detail; use their wood/stone for interior comparisons. |
| Floor IDs 16–21 and 32–37 | Retain the coherent nave/aisle/sanctuary flagstone families; do not regenerate the entire mixed floor atlas. |
| Storage props and harbor dock | Retain weathered wood/iron/rope treatment and complete silhouettes. |
| Shop badges IDs 0–6 | Retain readable approved symbols and matched frames; compare new public signs against these rather than changing interaction symbols. |

## Shared art direction

The game remains a weathered medieval-fantasy world. “More modern” means clearer, richer production quality. Buildings and props should share convincing worn materials and readable depth while retaining each region's climate, architecture and story identity.

Use a 32×32 map grid and consistent apparent pixel density. Upright objects use the established orthographic view from due south looking north, approximately 45 degrees downward, without yaw or isometric side faces. Floors use a directly overhead view. Keep front edges horizontal. Ground-contact anchors, object silhouettes and occupied footprints stay aligned to existing maps.

Use muted warm oak/pine, limestone/granite, tarnished iron/bronze and faded fabrics, sampled from the retained references. Lighting should read from the upper left with restrained highlights and crevice shadows. Keep snow/alpine cool tones, warm village timber, sanctuary ornament and deliberate magic accents distinct. Avoid bright cyan glass, smooth plastic-looking counters, modern fitted appliances and uniform orange varnish. Keep fine material texture readable at native scale; avoid blur and indiscriminate noise.

Generate complete objects or complete material families before slicing. A new roof must include its eaves, ridge, hips and repeat sections; a door must include both halves and its frame; a bridge must include caps, repeatable deck/rails and its coupled broken end. Use actual transparent PNG backgrounds for objects and opaque pixels for floor surfaces. Preserve the existing TSX projection, sorting properties and object anchors deliberately.

Every family should be shown as native-size and nearest-neighbor enlarged objects, seamless/repeated assemblies where relevant, and representative before/after map scenes over unchanged terrain-v7. Visual approval precedes committing that family. Changes to layout, portals, spawn markers, chest/NPC approaches or collisions require their own explicit validation. Keep prompts, generated source, packing layouts, exact model provenance and updated license hashes.

## Usage method

Placements count nonzero GIDs on visible tile layers and visible tile objects, with inherited visibility and zero opacity respected. Tiled transform flags are stripped before mapping each GID through that map's own firstgid table. The parser validates local IDs and CSV layer dimensions. It fails on unsupported map encodings instead of silently omitting them.

Hidden collision/spawn layers are excluded. Visible red collision-mask cells are inventoried as technical data and are never redesign targets or family-priority inputs. The shop template under `media/map_templates`, the editor sample under `media/tilesets`, and stamp recipes are excluded from player-exposure counts. Their source files are hashed for later migration review. A tile footprint with no nontransparent pixels is counted in the raw inventory but does not create a redesign target. There are 194 such authored placements outside terrain-v7, plus 4 inside the excluded terrain atlas.

Map count is the union of maps for the whole family, never the sum of individual tile map counts. Placement counts represent tile footprints rather than objects, screen time or visible pixels after occlusion. A large tree is several placements; an underlying floor may be covered by furniture. Runtime frequency and player traffic have not been measured.

The local plan ranks replacement families by **placements + 40 × distinct maps**. Forty tile placements of repeated area therefore equal one additional map of breadth. This is an explicit planning heuristic, not a claim about measured impact. The smaller harmonization pass follows the incompatible legacy families; zero-use artwork follows all live-use work. Hard dependencies can pull a supporting family forward before a higher-ranked integration.

## Atlas totals

“Art placements” includes only cells with visible source pixels; “footprints” includes transparent cells. Map counts in this table include all authored visible footprints in that atlas. A mixed atlas can contain both retained and redesign families; consult the per-tile CSV before changing pixels.

| Atlas | Art placements | Footprints | Maps | Nonempty cells |
| --- | ---: | ---: | ---: | ---: |
| `terrain-v7` | 97,604 | 97,608 | 28 | 1449 |
| `floor` | 9,094 | 9,124 | 67 | 90 |
| `wall` | 8,309 | 8,309 | 71 | 121 |
| `mountain_foothills` | 6,402 | 6,544 | 7 | 244 |
| `outdoor` | 3,553 | 3,553 | 22 | 94 |
| `collision` | 3,155 | 3,155 | 6 | 1 |
| `ancient_ruins` | 1,653 | 1,675 | 3 | 385 |
| `roof` | 1,338 | 1,338 | 6 | 67 |
| `ancient_ruins_props` | 1,166 | 1,166 | 3 | 57 |
| `shop` | 1,060 | 1,060 | 52 | 46 |
| `ancient_ruins_courtyard` | 921 | 921 | 2 | 160 |
| `interior` | 885 | 885 | 57 | 50 |
| `windows` | 354 | 354 | 50 | 18 |
| `ancient_ruins_borders` | 247 | 247 | 3 | 4 |
| `shrine_props` | 212 | 212 | 3 | 52 |
| `mountain_pass` | 130 | 130 | 2 | 106 |
| `ancient_ruins_sanctum` | 113 | 113 | 1 | 77 |
| `door` | 108 | 108 | 6 | 18 |
| `item_box` | 68 | 68 | 19 | 2 |
| `storage_props` | 43 | 43 | 3 | 6 |
| `harbor_dock` | 16 | 16 | 1 | 6 |
| `wood_stairs_up_right_32x32_v4` | 0 | 0 | 0 | 1 |
| `stone_stairs_up_right_32x32` | 0 | 0 | 0 | 1 |

## Refreshing the evidence

Run from the repository root with Python 3 and ImageMagick installed:

```sh
python3 temp/art/011-tile-style-audit/audit_usage.py
```

This refreshes `usage.csv` and `sources.json`. It does not regenerate the manually inspected screenshots/contact sheets or change the art classifications. Refresh those when their input hashes change. The refreshed family totals and map unions are derived from the saved CSV and cross-checked against the current local plan. The original October 24 snapshot was also checked against a separate first-pass parser. This planning audit does not claim that future redesigns have passed gameplay validation.

The recommended art model is `gpt-image-2.5-sunburst`, selected for reference-guided editing from [official OpenAI image-generation guidance](https://developers.openai.com/api/docs/guides/tools-image-generation), checked 2026-10-24. This is a model recommendation, not a benchmark of these assets. GPT-6.1 Sol at high effort is the suggested integration/validation model. Tool availability and the actual image backend must be resolved and recorded when a family is executed.

## Alternate plan source

The owner provided a [Claude artifact](https://claude.ai/artifact/3mh9QWxPWQZR2aDvCKSqp3) as the fallback when the local plan is unavailable. The local plan was accessible during this refresh. The web tool could not retrieve the artifact on 2026-10-26, so no claim is made that its content matches this revision.
