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
