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
- `fonts/Philosopher-Regular.ttf`: Philosopher font, licensed under the SIL Open
  Font License 1.1; see `fonts/Philosopher-OFL.txt`.

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
- `scenarios/rusted_kingdoms/assets/fonts/Philosopher-Regular.ttf` is the
  manifest-selected copy of Philosopher under the SIL Open Font License 1.1;
  its exact source notice is preserved beside it as `Philosopher-OFL.txt`.
- `scenarios/rusted_kingdoms/data/audio/bgm_index.yaml` maps Ardel's authored
  `town.default` key to
  `scenarios/rusted_kingdoms/assets/audio/bgm/Whiteveil_Streets.mp3`.
  `assets/audio/README-audio.md` preserves the source-tree generation prompt
  and YouTube source link, but does not provide a redistribution license. The
  local parity copy of this music is therefore a public-release blocker.
- `scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx` registers
  Aric's four-direction walk atlas and refers to the sibling
  `01_aric_walk.png` image.
- The Aric sprite is derived from Liberated Pixel Cup artwork and is
  distributed under CC BY-SA 3.0 and OGA-BY 3.0 for their respective
  components. Its per-layer creators, available license choices, and source
  links are preserved in
  `scenarios/rusted_kingdoms/credits/01_aric_credits.txt`; the complete audit,
  content hashes, and upstream history are recorded in
  `../docs/asset-license-inventory.md`.
- `scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx` is the canonical
  30-by-20 Ardel map. Its visible ground, terrain, and decoration layers use
  the copied `grass_cave_walls_24x14`, `icon_table_stage_14x9`, Astral Pixels
  `finestre`, and `ground/terrain-v7` TSX/PNG pairs. Collision-only atlas
  references are intentionally not runtime rendering dependencies.
- `scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt`
  preserves the complete LPC terrain attribution. The terrain atlas is
  distributed under CC BY-SA 3.0 with that notice. The byte-identical TMX
  copy remains blocked from public release until the project-authored map's
  redistribution grant is confirmed; see the license inventory.
- The source repository does not retain sufficient provenance or rights
  evidence for the copied `grass_cave_walls_24x14` and
  `icon_table_stage_14x9` images. The Astral Pixels notice identifies a
  plausible public asset page but does not prove the exact file's acquisition.
  These working-copy migrations are therefore blocked from public release as
  recorded in the license inventory.
