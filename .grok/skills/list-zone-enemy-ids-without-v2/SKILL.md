---
name: list-zone-enemy-ids-without-v2
description: >
  List enemy IDs used by each Rusted Kingdoms encounter zone that do not yet
  have a v2 status sprite. Use when asked for zone enemy coverage or IDs still
  needing v2 sprites.
---

# List zone enemy IDs without v2 sprites

Report encounter enemy IDs grouped by zone, excluding every ID that already has a v2 status sheet. Use the encounter YAML as the source of enemy IDs and the v2 sprite directory as the source of coverage.

## Procedure

1. Read every `zone_*.yaml` file in `assets/scenarios/rusted_kingdoms/data/encount/`. Skip town encounter files.
2. Collect unique enemy IDs from each `entries[].formation[]` and the optional `boss.id`. Do not treat the encounter file's own top-level `id` as an enemy.
3. For each ID, check for the exact file `assets/scenarios/rusted_kingdoms/media/sprites/enemies/v2/<id>-status.png`. Exclude IDs when that file exists. A matching attack sheet or unrelated filename is not enough to establish a v2 status sprite.
4. Sort IDs within each zone and report every zone, including zones with no uncovered IDs. Label those `(none)`.
5. State that these are the encounter IDs without v2 status sheets. Do not infer asset needs for enemies that are not in a zone encounter.

For a quick audit in this repository, this Python snippet reads the supported simple YAML patterns directly without adding dependencies:

```python
from pathlib import Path
import re

root = Path("assets/scenarios/rusted_kingdoms")
sprite_dir = root / "media/sprites/enemies/v2"
covered = {p.name.removesuffix("-status.png") for p in sprite_dir.glob("*-status.png")}

for path in sorted((root / "data/encount").glob("zone_*.yaml")):
    source = path.read_text()
    enemy_ids = set()
    for formation in re.findall(r"formation:\s*\[([^\]]*)\]", source):
        enemy_ids.update(re.findall(r"[a-z][a-z0-9_]*", formation))
    boss = re.search(
        r"^boss:\s*\n(?:[ \t]+.*\n)*?[ \t]+id:\s*([a-z][a-z0-9_]*)",
        source,
        re.MULTILINE,
    )
    if boss:
        enemy_ids.add(boss.group(1))
    missing = sorted(enemy_ids - covered)
    print(f"{path.stem}: {', '.join(missing) if missing else '(none)'}")
```

If encounter YAML structure changes, use a YAML parser or update the extraction before trusting the report. Confirm the ID extraction includes bosses and excludes the map's top-level ID.

## Current audit

Checked on 2026-11-12 against all 21 `zone_*.yaml` encounter files and the current `enemies/v2/*-status.png` files:

| Zone encounter | Enemy IDs without v2 status sprites |
| --- | --- |
| `zone_01_starting_forest` | (none) |
| `zone_02_open_plains` | (none) |
| `zone_02_open_plains_cave_01` | (none) |
| `zone_02_open_plains_cave_02` | (none) |
| `zone_03_marshland` | (none) |
| `zone_04_ancient_ruins_01_gate` | (none) |
| `zone_04_ancient_ruins_02_courtyard` | (none) |
| `zone_04_ancient_ruins_03_sanctum` | (none) |
| `zone_05_mountain_foothills_01` | `frankenstein_brawler_base`, `goblin_bomber_club_sapper`, `goblin_bomber_redcap_bomber`, `goblin_crossbowman_base`, `goblin_crossbowman_hooded_marksman`, `goblin_crossbowman_knife_backup` |
| `zone_05_mountain_foothills_02` | `frankenstein_brawler_base`, `goblin_bomber_club_sapper`, `goblin_bomber_redcap_bomber`, `goblin_crossbowman_base`, `goblin_crossbowman_hooded_marksman`, `goblin_crossbowman_knife_backup` |
| `zone_05_mountain_foothills_03` | `frankenstein_brawler_base`, `goblin_bomber_club_sapper`, `goblin_bomber_redcap_bomber`, `goblin_crossbowman_base`, `goblin_crossbowman_hooded_marksman`, `goblin_crossbowman_knife_backup` |
| `zone_06_mountain_pass_01` | `orc_archer_base`, `orc_archer_forest_bowman`, `orc_archer_greatbow_raider`, `orc_raider_shield_bruiser`, `orc_raider_spear_raider`, `orc_shaman_base`, `orc_shaman_bone_caller` |
| `zone_06_mountain_pass_01_house` | `orc_archer_base`, `orc_archer_forest_bowman`, `orc_archer_greatbow_raider`, `orc_raider_shield_bruiser`, `orc_raider_spear_raider`, `orc_shaman_base`, `orc_shaman_bone_caller` |
| `zone_06_mountain_pass_02` | `orc_archer_base`, `orc_archer_forest_bowman`, `orc_archer_greatbow_raider`, `orc_raider_shield_bruiser`, `orc_raider_spear_raider`, `orc_shaman_base`, `orc_shaman_bone_caller` |
| `zone_06_mountain_pass_03` | `orc_archer_base`, `orc_archer_forest_bowman`, `orc_archer_greatbow_raider`, `orc_raider_shield_bruiser`, `orc_raider_spear_raider`, `orc_shaman_base`, `orc_shaman_bone_caller`, `orc_shaman_red_hood_shaman` |
| `zone_07_sunken_cave` | `frankenstein_brawler_graveyard_smasher`, `lizard_monster_blue_scale_lizard`, `pirate_captain_base`, `pirate_captain_eyepatch_captain`, `skeleton_archer_hooded_archer`, `skeleton_monster_hooded_bones` |
| `zone_08_corrupted_forest` | `dark_fairy_redcap_fairy`, `pumpkin_jack_harvest_reaper`, `sheep_cultist_base`, `troll_berserker_base`, `troll_shaman_base`, `troll_shaman_swamp_hexer` |
| `zone_09_marshal_camp` | (none) |
| `zone_09_volcanic_region` | `boarman_berserker_base`, `boarman_berserker_blackhide_raider`, `lizard_dragon_guard_base`, `lizard_dragon_guard_blue_drake_guard`, `minotaur_brute_armored_minotaur`, `pigman_guard_base`, `pigman_guard_halberd_guard`, `wartotaur_warlord_blackhorn_chief` |
| `zone_10_final_stronghold` | `alien_invader_base`, `alien_invader_red_crystal_invader`, `fallen_angel_ash_wing`, `fallen_angel_base`, `fallen_angel_red_judicator`, `skeleton_knight_black_knight`, `skeleton_knight_mace_guard`, `troll_berserker_stone_troll`, `troll_shaman_crystal_sage`, `vampire_bat_lord_base`, `vampire_bat_lord_blood_saber_lord`, `vampire_bat_lord_white_wing_lord`, `vampire_noble_base`, `vampire_noble_blood_court_duelist`, `vampire_noble_masked_vampire`, `wartotaur_warlord_base` |
| `zone_10_hearth_core` | (none) |
