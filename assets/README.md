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
