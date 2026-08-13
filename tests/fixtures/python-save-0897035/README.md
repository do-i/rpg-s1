# Pinned Python save fixture

`007.yaml` was emitted by `GameStateManager.save` from source revision
`08970359d6cb03586948625d29b0d3351dbbf785` using the production Rusted
Kingdoms manifest, party, classes, and save manager. The clock was fixed at
`2026-08-12 15:30:45` and the constructed state contains Aric and Elise,
controlled Elise, flags, two repository stacks with tags/lock/loot metadata,
one visited map, one opened box, and 9,876 seconds of playtime.

The fixture was generated from the source checkout with its project virtual
environment. The essential invocation was:

```sh
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy .venv/bin/python <fixture-generator.py>
```

The generator loaded `rusted_kingdoms/manifest.yaml`, called
`from_new_game`, added Elise through `build_member`, applied the recorded
state including Starting Forest chest `forest_chest_01`, patched
`engine.io.save_manager.datetime.now`, and called
`GameStateManager.save(state, 7)`. The checked-in checksum is the serializer's
own PyYAML/CRC32 value, not a hand-authored approximation.

`converted-native-v1.yaml` is the deterministic native-envelope golden for
this input at the test's fixed conversion timestamp. It pins imported field
mapping, canonical ordering, deterministic RNG initialization, adapter/source
provenance, and native schema independently from the original Python file.
