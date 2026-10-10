# Migrated assets

Binary image, audio, and font files are stored with Git LFS. Install Git LFS
and run `git lfs install` before cloning or checking out the repository.

These prototype assets were copied from the sibling `agentic-rpg` project for
evaluating the Rust and Bevy direction.

- `images/title_lost_flame.webp`: title-screen artwork.
- `audio/title_theme.mp3`: title-screen music.
- `audio/menu_hover.mp3` and `audio/menu_confirm.mp3`: menu effects credited
  in the source repository to Leohpaz. Both are blocked from public release:
  the candidate store terms are documented, but exact-file provenance and a
  clear grant to embed the copied files in a distributable game are not yet
  evidenced. See `../docs/asset-license-inventory.md` for the required proof
  or replacement path.

The title artwork and title music are currently blocked from public release:
their audits did not find sufficient redistribution evidence. See
`../docs/asset-license-inventory.md` for the exact proof required to unblock or
replace each file.

## Rusted Kingdoms scenario assets

Scenario assets preserve their source-relative layout beneath
`scenarios/rusted_kingdoms/`.

- `scenarios/rusted_kingdoms/manifest.yaml`, `data/party.yaml`,
  `data/balance.yaml`, `data/dialogue/intro_cutscene.yaml`, and
  `data/maps/town_01_ardel.yaml` are the exact pinned scenario inputs loaded by
  the production new-game, intro, and initial-World path. These project-authored
  files remain blocked from public release pending an explicit redistribution
  grant; see the license inventory.
- The M5 field-dialogue subset under `data/dialogue/` is an exact copy of the
  source-authored Elise, Ardel NPC, elder, guide, and notice-board documents.
  These text assets remain blocked from public release under the same pending
  project-authored redistribution grant.
- `scenarios/rusted_kingdoms/media/fonts/Philosopher-Regular.ttf` is the
  manifest-selected copy of Philosopher under the SIL Open Font License 1.1;
  its exact source notice is preserved as
  `scenarios/rusted_kingdoms/credits/Philosopher-OFL.txt`.
- `scenarios/rusted_kingdoms/data/audio/bgm_index.yaml` maps Ardel's authored
  `town.default` key to
  `scenarios/rusted_kingdoms/media/audio/bgm/Whiteveil_Streets.mp3`.
  `media/audio/README-audio.md` preserves the source-tree generation prompt
  and YouTube source link, but does not provide a redistribution license. The
  local parity copy of this music is therefore a public-release blocker.
- `scenarios/rusted_kingdoms/media/sprites/party/01_aric_walk.tsx` registers
  Aric's four-direction walk atlas and refers to the sibling
  `01_aric_walk.png` image.
- The Aric sprite is derived from Liberated Pixel Cup artwork and is
  distributed under CC BY-SA 3.0 and OGA-BY 3.0 for their respective
  components. Its per-layer creators, available license choices, and source
  links are preserved in
  `scenarios/rusted_kingdoms/credits/01_aric_credits.txt`; the complete audit,
  content hashes, and upstream history are recorded in
  `../docs/asset-license-inventory.md`.
- The copied Elise and Ardel NPC TSX/PNG pairs provide the source-authored M5
  field cast. The sibling source identifies its character sprites generally as
  Liberated Pixel Cup assets, but retains no per-file generator credit for
  these exact hashes. They are local parity inputs and remain blocked from
  public release until exact layer provenance, license choices, and attribution
  are reconstructed or the sprites are replaced.
- `scenarios/rusted_kingdoms/media/maps/town_01_ardel.tmx` is the canonical
  30-by-20 Ardel map. Its visible ground, terrain, and decoration layers use
  the project-authored `outdoor`, `shop`, `town`, and `interior` sheets and the
  `ground/terrain-v7` TSX/PNG pairs. Collision-only atlas references are
  intentionally not runtime rendering dependencies.
- `scenarios/rusted_kingdoms/media/maps/town_01_ardel_house_01.tmx` and its
  same-stem map metadata provide the Gate 5 reversible interior. It draws from
  `tilesets/interior/interior.tsx`, a single 12-by-6 sheet of original tiles
  authored in this repository (MIT, like the rest of the project's own
  content). It replaced the eight Astral Pixels sheets on 2026-10-09, because
  that pack forbids redistribution; every map that used them now points here
  with the same layout.
- `scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt`
  preserves the complete LPC terrain attribution. The terrain atlas is
  distributed under CC BY-SA 3.0 with that notice. The byte-identical TMX
  copy remains blocked from public release until the project-authored map's
  redistribution grant is confirmed; see the license inventory.
- `tilesets/outdoor`, `town`, `shop`, `cave`, `exterior_walls`, and
  `exterior_windows` are original sheets authored in this repository (MIT).
  On 2026-10-10 they replaced `grass_cave_walls_24x14`,
  `stone_tile_stares_16x16`, `icon_table_stage_14x9`, the Schwarnhild cave
  pack, `walls_02`, and `window_8x6`, which either had no provenance or forbid
  redistribution. Each sheet keeps the old grid minus unused rows and
  columns, holding only the tiles the maps use.
- The M8 encounter package adds the Starting Forest encounter-zone document,
  all eight enemy-rank catalogs, the battle-background catalog, six enemy
  TSX/PNG pairs, the zone-one battle background, normal and boss battle BGM,
  and the encounter SFX. Their source-relative layout is preserved so the
  runtime resolves the same authored references as the pinned Python build.
- The enemy images are described generally by the source repository as LPC
  generator output, but exact per-file layer credits and license choices were
  not retained. The battle image and music likewise have no complete
  redistribution grant in the pinned tree. The encounter SFX credit names
  Leohpaz but does not grant redistribution rights for the copied file.
  Consequently every M8 copied asset remains a local parity input and public-
  release blocker until the evidence recorded in the license inventory is
  completed or the asset is replaced.
