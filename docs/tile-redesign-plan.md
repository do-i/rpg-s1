# Tile redesign plan — Rusted Kingdoms

Updated **2026-10-26** from the current worktree: **88 playable maps**, including the new Frostholm and Ashenveil rooms. This project tracks the plan locally; GitHub publication is not required. The deliverable here is the redesign plan. New art production proceeds one reviewed family at a time.

**31 remaining tasks** cover **14,199 authored tile placements**. The previously highest-ranked interior wall replacement is already implemented in commit `51b636f14568f58625774ae35b1b9207451f84d0` and is recorded below as a reference. Preserve the newer compatible ruins, alpine, shrine, flagstone, storage, dock and shop-badge art. `terrain-v7.png` and `terrain-v7.tsx` are excluded from redesign.

**Owner-provided fallback source:** [Claude artifact](https://claude.ai/artifact/3mh9QWxPWQZR2aDvCKSqp3). Use it when this local plan is unavailable. The artifact could not be retrieved through the web tool on 2026-10-26; this revision uses the accessible local source.

Current-state evidence: [audit and art direction](tile-style-audit/README.md), [per-tile usage and exact map membership](tile-style-audit/usage.csv), [source SHA-256 values](tile-style-audit/sources.json), [atlas comparison](tile-style-audit/legacy-atlases.png), and [recent reference atlas comparison](tile-style-audit/reference-atlases.png).

## Priority and models

Execute higher ranks first. Exposure score = **visible authored tile placements + 40 × distinct maps**. This balances repeated surface area and reuse across locations. It measures authored exposure rather than player traffic or screen time. Maps are counted once per family. Newer merchant display props need a smaller harmonization pass and follow incompatible legacy art; zero-use ritual seals come last. Hard dependencies take precedence over rank, including the cave floor that must be designed before its adjoining rock structure.

Each task uses **GPT Image 2.5 Sunburst (`gpt-image-2.5-sunburst`)** for reference-guided art, and **GPT-6.1 Sol, high effort** for atlas packing, metadata, migration and validation. The image-model assignment follows its documented editing role in [official OpenAI guidance](https://developers.openai.com/api/docs/guides/tools-image-generation), checked 2026-10-24. It is a recommendation, not a benchmark on these tiles. Record the actual backend and provenance during execution; the completed wall kit used the owner-approved built-in image tool with an undisclosed backend.

Effort is relative: **S** = a small object/family, **M** = a modular or related object family, **L** = several coordinated assemblies.

| Rank ↓ | Family | Placements | Maps | Exposure | Effort |
| ---: | --- | ---: | ---: | ---: | --- |
| 31 | [Seamless oak, pine and worn plank floor family](#wood-floors) | 3,829 | 50 | 5,829 | M |
| 30 | [Town facade and foundation kit](#exterior-walls) | 1,580 | 30 | 2,780 | L |
| 29 | [Merchant counter system](#counters) | 897 | 46 | 2,737 | M |
| 28 | [Living broadleaf tree family](#living-trees) | 2,261 | 10 | 2,661 | L |
| 27 | [Town and room window family](#windows) | 354 | 50 | 2,354 | M |
| 26 | [Household storage, bookshelves and cabinets](#shelves) | 280 | 34 | 1,640 | L |
| 25 | [Hearths, cooking counters, food and tableware](#kitchen) | 171 | 36 | 1,611 | M |
| 24 | [Modular town roof material family](#roofs) | 1,226 | 6 | 1,466 | L |
| 23 | [Timekeeping and mirror furnishings](#clocks-mirrors) | 120 | 32 | 1,400 | S |
| 22 | [Indoor potted plant family](#plants) | 71 | 33 | 1,391 | S |
| 21 | [Tables, chairs and stools](#seating) | 136 | 24 | 1,096 | M |
| 20 | [Beds and bedding family](#beds) | 84 | 23 | 1,004 | M |
| 19 | [Stairs and stairwell material family](#stairs) | 23 | 23 | 943 | S |
| 18 | [Public signs and direction boards](#signboards) | 34 | 21 | 874 | S |
| 17 | [Treasure chest closed and open states](#chests) | 68 | 19 | 828 | S |
| 16 | [Timber fence, posts and log barrier kit](#fences) | 579 | 6 | 819 | M |
| 15 | [Ground vegetation, flowers and mushrooms](#flowers) | 258 | 12 | 738 | M |
| 14 | [Ancient columns, lintels and broken stone supports](#columns) | 555 | 4 | 715 | M |
| 13 | [Hearth sandy stone floor](#sand-floor) | 388 | 7 | 668 | S |
| 12 | [Rope bridges and matching broken end](#bridges) | 315 | 4 | 475 | M |
| 11 | [Legacy outdoor grass, cliff, cave-mouth and dirt modules](#ground-overlays) | 353 | 2 | 433 | M |
| 10 | [Timber, painted and reinforced door family](#doors) | 108 | 6 | 348 | M |
| 9 | [Scarecrow and clothesline](#farm-props) | 20 | 5 | 220 | S |
| 8 | [Dead trees, stumps and fallen logs](#deadwood) | 40 | 4 | 200 | M |
| 7 | [Cave rock building and rubble object](#rock-building) | 127 | 1 | 167 | M |
| 6 | [Snow-covered roof kit](#snow-roofs) | 112 | 1 | 152 | M |
| 5 | [Bordered cave floor and rough ground](#cave-floor) | 97 | 1 | 137 | M |
| 4 | [Small blue stones and cave stepping-stone family](#stepping-stones) | 8 | 3 | 128 | S |
| 3 | [Well, grave markers and ground opening](#well-graves) | 4 | 2 | 84 | S |
| 2 | [Weapon/armor cabinets and merchant tools](#display-props) | 101 | 22 | 981 | M |
| 1 | [Dormant white and cyan ritual-circle variants](#runes) | 0 | 0 | 0 | S |

## Shared execution and acceptance

Use the retained references and the implemented wall kit for worn timber, carved/chipped stone, subdued metal and readable depth. Preserve each region’s architecture, climate and intentional magic accents. Use the established 32px map grid, consistent apparent pixel density, orthographic view from due south looking north with roughly 45-degree downward pitch for upright objects, and directly overhead floors. Keep clear pixel clusters, restrained upper-left lighting and horizontal front edges.

For every task:

1. Recheck its scope and current usage against the saved hashes. Generate and review whole objects or a complete material family before slicing. Keep the model, prompts, original generated output and deterministic packing layout in the project’s `temp/art/` workflow.
2. Preserve exact dimensions, tile IDs, geometry, anchors, transparency, transforms and sorting behavior where practical. Keep floors opaque and props on actual transparent alpha. If repacking is needed, remap all TMX, tile objects, templates, compressed stamps and runtime references while preserving flip flags.
3. Show native-size and enlarged before/after family sheets, assembled repeats and contextual map previews over unchanged terrain-v7. Obtain the owner’s visual review before committing the family. Retain approved neighboring art.
4. Preserve map layouts, portals, collision, chest/NPC approaches, routes and foreground sorting. Run relevant CSV/TSX/map checks and the scenario map sweep after integration; add reachability checks if footprints change. Refresh affected license hashes and make one scoped commit after review.
5. Verify both terrain-v7 files and every unrelated atlas region remain unchanged. Do not add placements or delete unused art to make a low-use task appear useful.

<a id="implemented-interior-walls"></a>
## Implemented reference: interior plaster and timber walls

**State:** implemented in commit `51b636f14568f58625774ae35b1b9207451f84d0`, followed by town/Starting Forest retiling in `00084d5`. Retain the current replacement rather than scheduling the original redraw again. Original rank: 32; remaining tasks are reranked below.

**Scope:** `wall.png`/`.tsx`, local IDs 0–11, 18–26 and 36–39. **Current usage:** 6,174 placements across 51 maps. **Actual art model:** undisclosed backend of the built-in image tool, explicitly recorded as an approved substitution in the existing provenance. [Historical generation and validation record](../temp/art/001-interior-walls/README.md). That record describes the October 24, 74-map validation; it is not a fresh test of the 88-map worktree. The commit and current source hashes supersede its old pending-commit note.

<a id="wood-floors"></a>
## Rank 31: Seamless oak, pine and worn plank floor family

**Usage:** 3,829 placements / 50 maps; exposure 5,829. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [implemented interior wall kit](#implemented-interior-walls)

**Why redesign:** Frequent smooth brown stripe textures lack the material depth of the recent timber props.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **0–12**.

**Deliverable:** Create a coherent family of worn oak/pine planks, edge variants and transitions. Keep species/value differences needed to distinguish rooms.

**Acceptance:** Show 8x8 repetitions of every floor, mixed joins and full-room views; no periodic stripe bands or brightness drift. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_shop_01`, `town_02_millhaven_house_01`, `town_03_harborgate_shop`, `town_04_frostholm_house_01`, `town_04_frostholm_house_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="exterior-walls"></a>
## Rank 30: Town facade and foundation kit

**Usage:** 1,580 placements / 30 maps; exposure 2,780. **Effort:** L. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Clean plaster rectangles and repeated brick faces lack the weathering and stone/timber volume of the newer buildings.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/wall` (.png and .tsx): local IDs **12–17, 30–35, 40–53, 58–86, 90, 92, 96–104, 108–109**.

**Deliverable:** Create timber/plaster, rough stone and plank facade modules, arches, foundation courses and matching corners. Preserve each town material identity.

**Acceptance:** Render all six town exteriors; doors sit on the ground, foundations join, and no open arch is accidentally filled. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel`, `town_03_harborgate`, `town_01_ardel_shop_01`, `town_02_millhaven_house_01`, `town_02_millhaven_house_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="counters"></a>
## Rank 29: Merchant counter system

**Usage:** 897 placements / 46 maps; exposure 2,737. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [implemented interior wall kit](#implemented-interior-walls); [rank 31: Seamless oak, pine and worn plank floor family](#wood-floors)

**Why redesign:** The flat gray counters and smooth orange island tops are a conspicuous modern-looking mismatch.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/shop` (.png and .tsx): local IDs **32–34, 36–38, 40–42, 44–46, 48–50, 53–54**.

**Deliverable:** Create a modular weathered oak merchant-counter kit with tops, fronts, side runs, corners, ends and lift hatch; retain usable counter gaps.

**Acceptance:** Preview straight, L, U and island arrangements plus every used hatch orientation; merchant approach and talk positions remain usable. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_04_frostholm_shop`, `town_01_ardel_shop_01`, `town_02_millhaven_house_01`, `town_02_millhaven_house_02`, `town_03_harborgate_house_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="living-trees"></a>
## Rank 28: Living broadleaf tree family

**Usage:** 2,261 placements / 10 maps; exposure 2,661. **Effort:** L. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Round faceted canopies and hard outlines read as simple cartoon trees next to detailed alpine pines and ruined woodland props.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **0–5, 16–21, 32–35, 48–51**.

**Deliverable:** Design complete small and broad trees in light, dark and autumn foliage, using the existing 2x2 footprints and shared bark treatment.

**Acceptance:** Inspect complete tree silhouettes in Starting Forest, Corrupted Forest and town views; preserve root baselines and canopy transparency. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_01_starting_forest`, `zone_08_corrupted_forest`, `zone_03_marshland`, `zone_07_sunken_cave`, `town_03_ruinwatch`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="windows"></a>
## Rank 27: Town and room window family

**Usage:** 354 placements / 50 maps; exposure 2,354. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [implemented interior wall kit](#implemented-interior-walls); [rank 30: Town facade and foundation kit](#exterior-walls)

**Why redesign:** Bright cyan panes, thick frames and simple reflection stripes feel more modern than the new stone/timber objects.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/windows` (.png and .tsx): local IDs **0–3, 5–8, 10–19**.

**Deliverable:** Create wood, painted timber, stone and arched window families plus curtained interiors; preserve paired tile layouts.

**Acceptance:** Check each full window against new plaster, timber and stone walls; no glass transparency exposing sky in interiors, no seam between halves. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_house_01`, `town_02_millhaven_mill`, `town_03_harborgate_harbormaster`, `town_04_frostholm_palace`, `town_05_ashenveil_chief_house`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="shelves"></a>
## Rank 26: Household storage, bookshelves and cabinets

**Usage:** 280 placements / 34 maps; exposure 1,640. **Effort:** L. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [implemented interior wall kit](#implemented-interior-walls); [rank 31: Seamless oak, pine and worn plank floor family](#wood-floors)

**Why redesign:** Shelf contents are tiny colored rectangles and all cabinet faces share flat frontal shading.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **0–10, 20–30**.

**Deliverable:** Create wardrobes, bookcases, linen/storage cupboards, drawers and potion/supply shelving as complete paired top/bottom objects.

**Acceptance:** Render complete furniture at native scale in homes, libraries and shops; readable contents, intact top/base joins and common wood palette. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_shop_item`, `town_02_millhaven_shop`, `town_03_harborgate_shop_item`, `town_03_ruinwatch_shop`, `town_04_frostholm_shop`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="kitchen"></a>
## Rank 25: Hearths, cooking counters, food and tableware

**Usage:** 171 placements / 36 maps; exposure 1,611. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [implemented interior wall kit](#implemented-interior-walls); [rank 29: Merchant counter system](#counters)

**Why redesign:** Flat black ovens, bright fire rectangles and slab-like worktops read as modern fitted kitchen units.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **45, 47–56**.

**Deliverable:** Create medieval stone/iron hearths, wooden preparation counters, a wash basin, bread/food surfaces, cookware and tableware.

**Acceptance:** Complete one coherent kitchen assembly; retain distinct flame/sink/preparation roles and current walkable approaches. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_house_01`, `town_02_millhaven_mill`, `town_03_harborgate_harbormaster`, `town_04_frostholm_palace`, `town_05_ashenveil_chief_house`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="roofs"></a>
## Rank 24: Modular town roof material family

**Usage:** 1,226 placements / 6 maps; exposure 1,466. **Effort:** L. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 30: Town facade and foundation kit](#exterior-walls)

**Why redesign:** Roof strips are bright and mechanically repetitive, with flatter eaves than the recent ruined cottage and alpine shelter.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/roof` (.png and .tsx): local IDs **0–2, 4–6, 8–10, 12–14, 16–18, 20–22, 24–26, 28–30, 32–34, 42–45, 47, 54–57, 59, 66–69, 71, 78–81, 83**.

**Deliverable:** Design hipped and repeatable wood-shingle, terracotta, slate, red diamond and thatch roofs, with ridge/eave/hip/end modules and coherent pitch.

**Acceptance:** Test three widths per material and both roof constructions; keep silhouette, building width, doorway clearance and regional colors. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_02_millhaven`, `town_03_harborgate`, `town_05_ashenveil`, `town_03_ruinwatch`, `town_01_ardel`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="clocks-mirrors"></a>
## Rank 23: Timekeeping and mirror furnishings

**Usage:** 120 placements / 32 maps; exposure 1,400. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 26: Household storage, bookshelves and cabinets](#shelves)

**Why redesign:** Bright cyan mirrors and uniform grandfather-clock cases need a less polished, more worn medieval-fantasy treatment.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **13–14, 33–34**.

**Deliverable:** Create a weathered carved timepiece or hourglass stand and tarnished framed mirror as paired top/bottom objects; preserve narrative identity if explicitly referenced.

**Acceptance:** Render full objects in affected rooms; mirror reflections subdued and no modern chrome/glass look. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_10_hearth_core`, `zone_10_hearth_descent_01`, `zone_10_hearth_descent_02`, `zone_10_hearth_descent_03`, `zone_10_hearth_descent_04`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="plants"></a>
## Rank 22: Indoor potted plant family

**Usage:** 71 placements / 33 maps; exposure 1,391. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Bright turquoise leaves and a simple striped pot stand out against the muted reference palette.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **46**.

**Deliverable:** Create readable 32px potted greenery with terracotta/aged ceramic pot variants and restrained leaf highlights.

**Acceptance:** Check plants against wood and stone floors in multiple room palettes; maintain compact collision silhouette. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_house_01`, `town_01_ardel_inn_01`, `town_01_ardel_shop_01`, `town_01_ardel_shop_item`, `town_01_ardel_shop_mc`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="seating"></a>
## Rank 21: Tables, chairs and stools

**Usage:** 136 placements / 24 maps; exposure 1,096. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 31: Seamless oak, pine and worn plank floor family](#wood-floors)

**Why redesign:** Thin green chairs and flat red tabletops have a different material and lighting language from the newer furniture.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **40–44, 58–59**.

**Deliverable:** Create complete dining/work tables, chair orientations and round table/stool objects, using one muted wood and fabric palette.

**Acceptance:** Verify seating orientation, usable aisle widths and assembled dining groups in homes and inns. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_shop_01`, `town_02_millhaven_house_01`, `town_03_harborgate_shop`, `town_04_frostholm_house_01`, `town_04_frostholm_house_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="beds"></a>
## Rank 20: Beds and bedding family

**Usage:** 84 placements / 23 maps; exposure 1,004. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 26: Household storage, bookshelves and cabinets](#shelves)

**Why redesign:** Repeated narrow beds use flat blue diamond blankets and abrupt headboard joins.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **11–12, 31–32**.

**Deliverable:** Create complete oak beds with muted wool/linen bedding and matching headboards/footboards in the existing two-tile footprints.

**Acceptance:** Preview adjacent inn beds and single home beds; north/south halves join and bed bases remain readable. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_inn_01`, `town_01_ardel_shop_01`, `town_02_millhaven_house_01`, `town_02_millhaven_inn`, `town_03_harborgate_inn`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="stairs"></a>
## Rank 19: Stairs and stairwell material family

**Usage:** 23 placements / 23 maps; exposure 943. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** The live indoor stairwell and standalone alternatives need the same worn stone/timber palette and perspective; the standalone alternatives have no current map references.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/interior` (.png and .tsx): local IDs **57**.
- `assets/scenarios/rusted_kingdoms/media/tilesets/stone_stairs_up_right_32x32` (.png only): local IDs **0**.
- `assets/scenarios/rusted_kingdoms/media/tilesets/wood_stairs_up_right_32x32_v4` (.png only): local IDs **0**.

**Deliverable:** Create an interior stairwell and matching stone/timber ascending-right stair variants together, preserving accepted silhouettes and rise/run geometry.

**Acceptance:** Compare alpha masks and joins with their accepted anchors; verify exact 32x32 dimensions and no unintended live map changes. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_house_01`, `town_01_ardel_shop_armor`, `town_01_ardel_shop_item`, `town_01_ardel_shop_weapon`, `town_02_millhaven_mill`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="signboards"></a>
## Rank 18: Public signs and direction boards

**Usage:** 34 placements / 21 maps; exposure 874. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Bright tan flat placards and mechanical framing are less weathered than the new settlement props.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **12–15**.

**Deliverable:** Create small/large message boards, notice board and direction sign as one readable weathered sign family; retain interaction identity.

**Acceptance:** Verify silhouettes at 1x, sign dialogue lookup IDs and all existing sign positions; do not introduce unreadable decorative text. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_03_harborgate`, `town_04_frostholm`, `town_05_ashenveil`, `zone_02_open_plains`, `zone_02_open_plains_cave_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="chests"></a>
## Rank 17: Treasure chest closed and open states

**Usage:** 68 placements / 19 maps; exposure 828. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** The tiny bright yellow chest and strong black outline are simpler than nearby new wooden props; chest-state recognition must survive the redesign.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/sprites/objects/item_box` (.png and .tsx): local IDs **0–1**.

**Deliverable:** Create closed/open 32px weathered oak and dull iron chest states as a paired object family.

**Acceptance:** Preview both states in forest, ruin and interior scenes; remain distinguishable at native scale and preserve chest interaction and collision behavior. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_01_starting_forest`, `zone_02_open_plains`, `zone_02_open_plains_cave_01`, `zone_02_open_plains_cave_02`, `zone_03_marshland`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="fences"></a>
## Rank 16: Timber fence, posts and log barrier kit

**Usage:** 579 placements / 6 maps; exposure 819. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Pale smooth cylindrical posts and bright rails contrast with the weathered broken fence reference.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **64–69, 80–85**.

**Deliverable:** Create intact rustic rails, junctions, end posts, branch-style barriers and upright log variants; reuse the new broken fence material language.

**Acceptance:** Test long runs, corners, T junctions and short closures in all affected towns; preserve passable gaps and blocked footprints. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_04_frostholm`, `town_02_millhaven`, `town_03_harborgate`, `town_03_ruinwatch`, `town_01_ardel`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="flowers"></a>
## Rank 15: Ground vegetation, flowers and mushrooms

**Usage:** 258 placements / 12 maps; exposure 738. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Candy-colored flowers and rounded shrubs are small but repeat across several routes and settlements.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **8–11, 24–27, 56–57, 63**.

**Deliverable:** Create bushes, grass tufts, mushrooms, loose leaves, lily pads and flower color variants as one restrained ground-cover family.

**Acceptance:** Inspect at native scale over unchanged terrain-v7 grass/dirt/water; flowers retain color cues without a neon or icon-like look. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel`, `town_03_harborgate`, `town_03_ruinwatch`, `town_02_millhaven`, `town_04_frostholm`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="columns"></a>
## Rank 14: Ancient columns, lintels and broken stone supports

**Usage:** 555 placements / 4 maps; exposure 715. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Pale ornamental columns and blue inlays clash with mossy, chipped ruins; narrow repeated fragments need consistent material scale.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/wall` (.png and .tsx): local IDs **114, 126–127, 129–130, 132, 134–136, 138, 140, 142, 144–145, 147–148, 150, 152–153, 156, 158, 162–163, 165–166, 168, 170, 176**.

**Deliverable:** Create intact and damaged column/lintel sets using the newer ancient-ruins stone language while retaining sanctuary ornament and any required blue wards.

**Acceptance:** Assemble every column/lintel template in Ancient Ruins and Hearth maps; tops, shafts, bases and damaged pieces read as complete objects. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_04_ancient_ruins_01_gate`, `zone_10_final_stronghold`, `town_01_ardel_shrine`, `town_01_ardel_epilogue`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="sand-floor"></a>
## Rank 13: Hearth sandy stone floor

**Usage:** 388 placements / 7 maps; exposure 668. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** The single pale octagonal sandy cell looks like a repeated marker when used as a full dungeon floor.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **22**.

**Deliverable:** Create an opaque seamless warm mineral/sandstone floor matching the Hearth environment, with restrained grit and heat-aged variation.

**Acceptance:** Render Hearth Core and one descent room at native scale; seamless 8x8 repeat, visible route and encounter-marker readability. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_10_hearth_core`, `zone_10_hearth_descent_01`, `zone_10_hearth_descent_02`, `zone_10_hearth_descent_03`, `zone_10_hearth_descent_04`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="bridges"></a>
## Rank 12: Rope bridges and matching broken end

**Usage:** 315 placements / 4 maps; exposure 475. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** The legacy pale flat deck and perfectly straight ropes are simpler than the new stone bridges; the broken cap is coupled to those source pixels.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **45–47, 61–63, 77–79, 93–95, 109–111, 125–127**.
- `assets/scenarios/rusted_kingdoms/media/tilesets/mountain_pass` (.png and .tsx): local IDs **9–11, 21–23**.

**Deliverable:** Create horizontal and vertical rope-bridge kits together with the broken right end. Keep endpoint geometry, deck width, rails and current spans.

**Acceptance:** Verify exact repeated seams and cap joins, including the preserved-source seam of the Zone 06 broken bridge; check all four flip/rotation uses. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_02_open_plains_cave_01`, `zone_06_mountain_pass_02`, `zone_05_mountain_foothills_01`, `zone_05_mountain_foothills_02`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="ground-overlays"></a>
## Rank 11: Legacy outdoor grass, cliff, cave-mouth and dirt modules

**Usage:** 353 placements / 2 maps; exposure 433. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** The remaining non-terrain-v7 cliff/dirt set has a coarse brown vertical face and sharply square grass tops.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **70–72, 75, 86–88, 90–91, 96–97, 101–104, 107, 117–120, 122, 134–136**.

**Deliverable:** Create a matching legacy overlay kit only in outdoor.png: grass/dirt edges, rocky patch, cliff face/corners and cave entrance. Use terrain-v7 as immutable visual reference.

**Acceptance:** Test every edge/corner and cave-mouth assembly in affected maps; terrain-v7 PNG, TSX and existing canonical IDs are byte-identical. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_02_open_plains`, `zone_01_starting_forest`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="doors"></a>
## Rank 10: Timber, painted and reinforced door family

**Usage:** 108 placements / 6 maps; exposure 348. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 30: Town facade and foundation kit](#exterior-walls)

**Why redesign:** Smooth saturated blue/red panel doors look contemporary beside the worn stone/plaster architecture.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/door` (.png and .tsx): local IDs **0–16, 24**.

**Deliverable:** Create plank, arched, painted, stone-jamb, round-window and iron-banded doors with muted paint and hand-forged hardware, including unused matching variants.

**Acceptance:** Preview every complete two-tile door in its frame; all portal trigger positions, opening widths and foot alignment remain unchanged. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_02_millhaven`, `town_03_harborgate`, `town_04_frostholm`, `town_05_ashenveil`, `town_03_ruinwatch`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="farm-props"></a>
## Rank 9: Scarecrow and clothesline

**Usage:** 20 placements / 5 maps; exposure 220. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Flat bright cloth and simple stick construction differ from newer weathered props.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **44–46, 60–62**.

**Deliverable:** Create one rustic farm/household cloth set: scarecrow and clothesline with complete paired top/base sections.

**Acceptance:** Check full objects in Millhaven and the laundry placement; preserve cloth clearance and correct anchors. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_03_ruinwatch`, `town_02_millhaven`, `town_03_harborgate`, `town_04_frostholm`, `town_05_ashenveil`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="deadwood"></a>
## Rank 8: Dead trees, stumps and fallen logs

**Usage:** 40 placements / 4 maps; exposure 200. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 28: Living broadleaf tree family](#living-trees)

**Why redesign:** Thin branch outlines and round cut stumps differ from the detailed gnarled ruins dead-tree reference.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **6–7, 22–23, 38–43**.

**Deliverable:** Create dead-tree, small/large stump and horizontal fallen-log family using the newer ruins bark palette within existing footprints.

**Acceptance:** Assemble all complete objects; keep walk-behind behavior, tree sorting baselines and horizontal log joins. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_03_marshland`, `zone_07_sunken_cave`, `town_01_ardel`, `town_02_millhaven`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="rock-building"></a>
## Rank 7: Cave rock building and rubble object

**Usage:** 127 placements / 1 maps; exposure 167. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 5: Bordered cave floor and rough ground](#cave-floor)

**Why redesign:** Although recently repaired, this object intentionally inherits a coarse older palette and still differs from the new ruins stone.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **83–85, 99–101, 115–117, 131–133**.

**Deliverable:** Design the complete 3x4 rock structure with connected cap, sides, inset center and rubble base, using the newer weathered stone language.

**Acceptance:** Preserve footprint and rubble silhouette; verify every shared row/column at joins and all existing placements in Open Plains Cave 01. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_02_open_plains_cave_01`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="snow-roofs"></a>
## Rank 6: Snow-covered roof kit

**Usage:** 112 placements / 1 maps; exposure 152. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 24: Modular town roof material family](#roofs)

**Why redesign:** Regular snow stripes look painted onto tile rows instead of accumulating on roof surfaces.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/roof` (.png and .tsx): local IDs **36–40, 48–52, 60–64, 72–76**.

**Deliverable:** Create snow-capped eaves, slopes, ridge and repeating snow-density variants aligned with the main roof kit and alpine shelter reference.

**Acceptance:** Render Frostholm and a repeat strip with at least three widths; snow follows pitch and repeated patches do not form horizontal bands. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_04_frostholm`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="cave-floor"></a>
## Rank 5: Bordered cave floor and rough ground

**Usage:** 97 placements / 1 maps; exposure 137. **Effort:** M. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Speckled brown ground and heavy frame borders remain coarse beside the newer dungeon masonry.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **80–82, 96–98, 112–114, 128–130**.

**Deliverable:** Create an opaque bordered cave-floor kit and matching loose-ground tiles, preserving current straight inner-border geometry and cave layout.

**Acceptance:** Repeat center tiles and assemble every 3x3 border combination; preserve opacity and joins under the rock building. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_02_open_plains_cave_01`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="stepping-stones"></a>
## Rank 4: Small blue stones and cave stepping-stone family

**Usage:** 8 placements / 3 maps; exposure 128. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Uniform blue square blocks read as interface buttons rather than small physical stepping stones.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **88–89, 104–105**.
- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **58–59**.

**Deliverable:** Create chipped dark slate stepping stones as a complete small crossing kit; preserve any intentional blue mineral accent. Include the two small blue outdoor stones in the same slate/mineral family.

**Acceptance:** Inspect all placements in Open Plains Cave 01; stones read as walkable rock at native scale and existing paths remain readable. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `zone_02_open_plains_cave_01`, `zone_10_final_stronghold`, `town_01_ardel`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="well-graves"></a>
## Rank 3: Well, grave markers and ground opening

**Usage:** 4 placements / 2 maps; exposure 84. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Small outlined well/grave/pit icons lack the material and depth cues used by newer stone props.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **28–31**.

**Deliverable:** Create complete village well, carved grave/cross and ground opening assets together as a small stone site-prop family.

**Acceptance:** Verify material scale, opening readability and the affected village/route context; grave symbols retain narrative meaning. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel`, `town_03_ruinwatch`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="display-props"></a>
## Rank 2: Weapon/armor cabinets and merchant tools

**Usage:** 101 placements / 22 maps; exposure 981. **Effort:** M. **Type:** harmonize.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** [rank 29: Merchant counter system](#counters); [rank 26: Household storage, bookshelves and cabinets](#shelves)

**Why redesign:** These newer props are usable references, but glossy edges and yellow wood can be harmonized after the shared architecture is established.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/shop` (.png and .tsx): local IDs **8–14, 16–22, 24–31**.

**Deliverable:** Retune cabinet wood/metal and produce any necessary matched cabinet, armor stand, anvil, sword barrel, shield, potion shelf, scales, ingot crate and lantern variants as one workshop/display family.

**Acceptance:** Keep weapon/armor/tool identity and full object silhouettes; compare native-scale shop scenes with new counters and shelves. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** `town_01_ardel_shop_weapon`, `town_02_millhaven_shop_weapon`, `town_03_harborgate_shop_weapon`, `town_03_ruinwatch_shop_weapon`, `town_04_frostholm_shop_weapon`. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.

<a id="runes"></a>
## Rank 1: Dormant white and cyan ritual-circle variants

**Usage:** 0 placements / 0 maps; exposure 0. **Effort:** S. **Type:** redesign.

**AI models:** `gpt-image-2.5-sunburst` for art; GPT-6.1 Sol (high effort) for integration and checks.

**Dependencies:** None.

**Why redesign:** Thin luminous geometric circles do not match the worn carved-stone magic language, but currently have no visible map placements.

**Exact scope** (zero-based local IDs, not global GIDs):

- `assets/scenarios/rusted_kingdoms/media/tilesets/floor` (.png and .tsx): local IDs **39–44, 55–60, 71–76**.
- `assets/scenarios/rusted_kingdoms/media/tilesets/outdoor` (.png and .tsx): local IDs **47**.

**Deliverable:** Create paired white/cyan ritual seals with restrained weathering and readable shared geometry if these assets are activated. Include the unused small outdoor magic-circle tile as a matching compact seal.

**Acceptance:** Verify full 3x3 assemblies, circle continuity and 1x legibility; do not add placements simply to justify this low-priority work. Also satisfy every shared execution and acceptance criterion above.

**Representative maps:** No current gameplay placements. Keep this last until an activation decision is made.. Exact membership is in the saved per-tile CSV.

- [ ] Review whole-object/family direction and produce the complete asset family.
- [ ] Integrate the approved family and attach contextual before/after evidence.
- [ ] Pass the family-specific and shared checks; make the scoped commit.
