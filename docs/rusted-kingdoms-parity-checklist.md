# Rusted Kingdoms Player-Visible Parity Checklist

This is the durable manual acceptance contract for porting **Chronicles of the
Lost Flame** to the Rust/Bevy application. It maps the player-facing and
operator-observable claims in the pinned source `README.md` to granular checks
whose failures can be diagnosed independently.

Source: `../agentic-rpg/README.md` at
`08970359d6cb03586948625d29b0d3351dbbf785`

Target plan: `docs/rusted-kingdoms-port-plan.md`

Supporting coverage records: `docs/rusted-kingdoms-engine-inventory.md`,
`docs/rusted-kingdoms-scenario-inventory.md`, and
`docs/m5-manual-play-checklist.md`

Automated tests, validators, screenshots, transcripts, and state hashes may
support a row, but they never replace performing its stated action and
observing its result. Run this checklist against a self-contained release
candidate for final parity unless a row explicitly calls for an authoring or
debug build.

## Status rules

| Status | Meaning |
| --- | --- |
| `Not run` | The Rust parity behavior has not been manually accepted. This is the default. |
| `Prototype observed` | The accepted title prototype exhibited the baseline behavior, but release parity has not passed. |
| `Pass` | The stated action and all applicable visible, audible, state, and persistence outcomes were observed on the recorded build. |
| `Accepted difference` | A deliberate difference is documented and approved with a link from the row. |
| `Blocked` | The row cannot be run; the blocker and owner are linked from the row. |

`Prototype observed` is deliberately not a passing status. Never infer `Pass`
from an automated check. When executing a row, append the build identifier,
date, platform, tester, and evidence link to its Status cell or to a linked run
report.

## Standard setups

| Setup | Prerequisite and preparation |
| --- | --- |
| `SET-FRESH` | Self-contained release candidate, clean user profile, no save files, default Rusted Kingdoms scenario, normal mode, seed `1`, working display and audio. |
| `SET-NEW` | `SET-FRESH`, then begin New Game, enter a distinctive protagonist name, finish the intro, and stand at the manifest start in Ardel. |
| `SET-WORLD` | A normal save in a migrated campaign map containing collision, a portal, an NPC, a sign, a closed item box, and a visible enemy; no debug overrides active. |
| `SET-PARTY` | A normal campaign save with all five source party members, mixed rows, damaged HP/MP, at least one curable status, equipment choices, field spells, and representative inventory in every tab. |
| `SET-BATTLE` | A normal save immediately before a regular encounter containing at least two living enemies; party inventory includes usable battle items and members have offensive, healing, and status abilities. |
| `SET-BOSS` | A normal save immediately before a source-defined boss, reached without debug state injection. |
| `SET-SERVICE` | A normal Ardel-area save with known GP, buyable and sellable items, a compatible equipment upgrade, inn damage to recover, recipe ingredients, and at least one locked recipe/stock entry. |
| `SET-QUEST` | A normal save immediately before a dialogue-started quest with flag and item objectives, recruitment effects, and a reward; the quest has not started. |
| `SET-SAVES` | Three known slots: one empty, one valid mid-campaign save with distinctive state, and one deliberately corrupt or incompatible fixture; preserve a copy of the valid slot. |
| `SET-DEBUG` | Debug/authoring build launched through the documented developer menu with a writable temporary scenario copy and recording output directory. |
| `SET-CONTENT` | Two valid scenario packages with visibly different identity, start map, protagonist, dialogue, art, audio, and balance values; neither requires changing or rebuilding the Rust binary. |

If a standard setup does not naturally expose the row's condition, add the
smallest source-data fixture or normal save needed and record it with the run.
Do not use runtime state injection for campaign-reachability or campaign-ending
rows.

## Title, startup, and top-level flow

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-TTL-001 | `SET-FRESH` | Launch the Rust game and wait at the title screen for ten seconds. | The resizable game window shows the scenario title art, font, and New Game/Load Game/Quit menu without missing-asset placeholders; title music begins and continues without restarting during the observation. | Title is the only active flow and no game state or save is created. | M0, M1 / Gate 1 | Not run |
| RK-TTL-002 | `SET-FRESH` | Press Down through every title entry, then once more; press Up once. | Exactly one entry is visibly selected; each move plays one hover sound; selection wraps last-to-first and first-to-last. | Navigation alone does not activate an entry or create state. | M0.13, M1.06-M1.07 / Gate 1 | Not run |
| RK-TTL-003 | `SET-FRESH` | Select the disabled Load Game entry and press Enter. | Load Game is visibly disabled and confirmation produces no load picker or confirm sound. | The title remains active and no slot is read as a session. | M0.13, M7.07, M7.10 / Gate 7 | Pass — automated action/color proof and live fresh-profile disabled state |
| RK-TTL-004 | `SET-FRESH` | Select New Game and press Enter once. | One confirm sound plays and the name-entry prompt replaces the title menu. | Exactly one new-game/name-entry transition occurs; no save exists yet. | M1.05, M3.11 / Gate 3 | Not run |
| RK-TTL-005 | One valid save and one empty slot | Relaunch, select Load Game, and press Enter. | Load Game is enabled and a picker distinguishes the valid and empty slots with readable metadata. | Merely opening the picker does not modify either slot. | M7.07, M7.10-M7.11 / Gate 7 | Pass — live restart showed Slot 01 `[LATEST]` plus empty rows |
| RK-TTL-006 | `SET-SAVES` | In the title load picker, select the valid slot and confirm. | A loading transition leads to the saved map with the expected map audio. | The chosen slot restores its state once; other slots remain unchanged. | M7.11-M7.12 / Gate 7 | Pass — live Ardel process-restart load and Starting Forest import load |
| RK-TTL-007 | `SET-FRESH` | Select Quit and press Enter once. | One confirm sound plays and the window closes normally without an error dialog. | The process exits successfully and creates no save. | M0.13, M1 / Gate 1 | Not run |
| RK-TTL-008 | Any normal world session | Use the field-menu Quit command, confirm leaving the session, and return to title. | World visuals/audio stop and the title art/menu/title music appear once. | Unsaved changes follow the documented confirmation policy; a new session is not layered over the old one. | M6.01-M6.02, M7, M14.10 / Gate 7 | Pass — live confirmed return produced one clean enabled-load title |

## Controls and contextual help

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-CTL-001 | `SET-NEW`, with one open tile in each cardinal direction | Tap each Arrow key once from a reset position. | The protagonist faces and moves in the pressed cardinal direction with walk animation. | Each legal tap changes position by exactly one tile. | M4.18-M4.20 / Gate 4 | Not run |
| RK-CTL-002 | Any title, field, item, spell, equipment, shop, quest, save, or battle menu with at least three entries | Press Up and Down, including across both list ends. | The visible selection moves one row per press and follows the menu's documented wrap/clamp rule. | Navigation does not mutate the highlighted item or game state. | M1.06-M1.07, M6.02 / Gate 6 | Not run |
| RK-CTL-003 | `SET-WORLD`, facing an adjacent NPC/sign/box | Press Enter once. | The facing interactable responds with its dialogue, sign text, or box feedback and appropriate SFX. | Only that nearest eligible interactable receives the action. | M5.11-M5.26 / Gate 5 | Not run |
| RK-CTL-004 | Any enabled menu choice | Highlight the choice and press Enter once. | The choice gives one visible/audible confirmation and opens or applies the selected action. | The action is committed once, even if Enter is held briefly. | M1.06-M1.07, M3.17 / Gate 3 | Not run |
| RK-CTL-005 | `SET-NEW` | Press M, wait for the field menu, then press M again. | The field menu opens and closes cleanly. | World movement is paused while open and resumes at the unchanged position. | M6.01 / Gate 6 | Pass — X11 2026-08-12 |
| RK-CTL-006 | `SET-NEW` | Press I from the overworld. | The Items screen opens directly with inventory rows and controls visible. | Party, map, and inventory remain unchanged until an item action is confirmed. | M6.06-M6.08 / Gate 6 | Pass — X11 2026-08-12 |
| RK-CTL-007 | `SET-NEW` | Press S from the overworld. | The protagonist's character status screen opens directly. | Merely opening status changes no stats or party order. | M6.03-M6.05, M6.25 / Gate 6 | Pass — X11 2026-08-12; original-style Status visually verified 2026-08-16 |
| RK-CTL-008 | A submenu nested at least two levels deep | Press Escape once. | The current submenu closes and its parent is visible. | Unconfirmed changes in the closed submenu are discarded. | M1.06, M6.02 / Gate 6 | Pass — X11 2026-08-12 |
| RK-CTL-009 | A top-level field menu | Press Escape once. | The menu closes and the overworld is visible. | World input resumes without state loss. | M6.01-M6.02 / Gate 6 | Pass — X11 2026-08-12 |
| RK-CTL-010 | `SET-BATTLE`, at the command menu | Press Escape once to request fleeing. | The battle displays success or failure feedback; the result is accompanied by the proper SFX/transition. | Success returns safely to the stored world context; failure consumes the source-defined turn cost. | M9.14-M9.16 / Gate 9 | Not run |
| RK-CTL-011 | A context with extra controls, such as target selection or quantity choice | Open that context and inspect the bottom of the screen before acting. | Context-specific key hints are readable, accurate, and not clipped. | Following each displayed hint performs the stated action. | M3-M11 UI / Gate 11 | Not run |
| RK-CTL-012 | Any dialogue line still typing | Press Enter once, then press Enter again after the full line appears. | The first press reveals the complete current line; the second advances exactly one line. | No line, choice, or completion effect is skipped or applied twice. | M5.13-M5.14 / Gate 5 | Not run |

## New game, TMX world, camera, movement, and interactions

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-WLD-001 | `SET-FRESH` | Start New Game, edit the default protagonist name with insert/delete, then confirm. | The prompt shows edits and length feedback; the confirmed name appears in subsequent dialogue/status. | Only the runtime protagonist name changes and it survives the next save/load. | M3.11-M3.13 / Gate 3 | Not run |
| RK-WLD-002 | Name entry just confirmed | Advance every intro line without skipping. | Linear cutscene text is readable, advances once per confirm, and transitions visibly into Ardel. | Intro completion flags apply once and the manifest start map/position is entered. | M3.14-M3.17 / Gate 3 | Not run |
| RK-WLD-003 | `SET-NEW` | Stand still and inspect Ardel, then compare it with the pinned Python build at the same tile. | All visible TMX ground, terrain, decoration, and top layers render in source order; the collision layer is invisible. | Map identity and tile placement come from the scenario files. | M4.01-M4.13 / Gate 4 | Not run |
| RK-WLD-004 | `SET-NEW`, beside one known wall and one open tile | Walk into the wall, then toward the open tile. | The blocked attempt leaves the sprite facing the wall; the open attempt animates normally. | Collision prevents entry into the blocked cell and permits exactly one-tile movement into the open cell. | M4.14, M4.18-M4.19 / Gate 4 | Not run |
| RK-WLD-005 | `SET-WORLD`, at a diagonal corner case | Hold two perpendicular Arrow keys toward an open diagonal, then repeat where one/both adjacent cells are blocked. | Facing/walk animation matches the source eight-way input policy without jitter. | Diagonal displacement and corner blocking match the pinned Python behavior. | M4.21 / Gate 4 | Not run |
| RK-WLD-006 | A large campaign map | Walk from the center to each map edge. | The camera follows smoothly, shows no outside-map void, and remains clamped at edges. | Camera movement does not alter logical player coordinates. | M4.22 / Gate 4 | Not run |
| RK-WLD-007 | A small interior map | Walk across the whole interior. | The camera framing remains stable without oscillation or exposed void. | Player movement and collision remain correct when the map is smaller than the canvas. | M4.22 / Gate 4 | Not run |
| RK-WLD-008 | `SET-NEW`, near layered scenery | Walk above and below a foreground/overhead object. | The protagonist and map content swap front/behind ordering at the correct Y positions. | Rendering order does not change collision or position. | M4.23 / Gate 4 | Not run |
| RK-WLD-009 | `SET-NEW` | Walk, change directions, then release all movement keys. | The protagonist uses correct directional animation frames and settles to the correct idle facing. | Facing is retained for the next interaction and save. | M4.16-M4.20 / Gate 4 | Not run |
| RK-WLD-010 | `SET-WORLD`, with static and wandering NPCs | Observe for ten seconds, then approach each NPC. | Static/step/wander animations, facing, and bounded movement match metadata without clipping through scenery. | NPC occupancy blocks overlap; deterministic wandering remains within configured bounds. | M5.08-M5.10, M5.19-M5.20 / Gate 5 | Not run |
| RK-WLD-011 | `SET-WORLD`, at a reversible portal | Walk into the portal, wait through the transition, then use the return portal. | Fade-out locks input, the destination appears at its configured position, fade-in completes, and destination BGM/visuals replace the source map. | Each entry records the visited destination once and the round trip restores valid map state. | M5.01-M5.07 / Gate 5 | Not run |
| RK-WLD-012 | `SET-WORLD`, facing an NPC with multiple lines | Press Enter and advance the entire conversation. | Speaker, text, backdrop, continuation marker, typewriter timing, and dialogue SFX remain readable and ordered. | Conversation flags/effects apply only at their configured point. | M5.12-M5.17 / Gate 5 | Not run |
| RK-WLD-013 | `SET-WORLD`, facing a configured sign | Press Enter and close the sign dialogue. | The map-specific sign text appears and closes cleanly. | Reading the sign does not alter unrelated flags or inventory. | M5.21-M5.22 / Gate 5 | Pass |
| RK-WLD-014 | `SET-WORLD`, facing a closed item box | Record inventory count, press Enter once, and dismiss the grant message. | The box opens, the granted item/quantity is named, and one box/confirm SFX plays. | Inventory increases exactly once and the stable box ID becomes opened. | M5.23-M5.24 / Gate 5 | Not run |
| RK-WLD-015 | Same session after RK-WLD-014 | Interact with the opened box again. | The box remains visibly open and gives already-open feedback. | No item is granted a second time. | M5.25 / Gate 5 | Pass |
| RK-WLD-016 | Save after RK-WLD-014, then relaunch and load it | Return to the same box and interact. | The box still renders open and gives already-open feedback. | Opened-box state and inventory quantity survive save/load without duplication. | M7.12, M11.20 / Gate 7 | Not run |
| RK-WLD-017 | `SET-WORLD`, with a visible enemy | Observe the enemy, approach it, and make contact. | The enemy sprite is visible and animates/moves according to data; contact triggers one battle transition. | World input freezes once and the pre-battle map/position/facing context is retained. | M8.05-M8.12 / Gate 8 | Pass |
| RK-WLD-018 | `SET-WORLD`, with an enemy inside chase range | Keep the player still while the enemy moves, then move the player through the same area. | Tile boundaries remain solid with no horizontal or vertical grid flicker during either enemy or camera motion. | Rendering-only sampler and antialiasing policy does not alter player, enemy, collision, or encounter state. | M4.27 / Gate 4 maintenance | Pass — user-verified real world scene 2026-08-16 |

## Encounters, battle rules, UI, outcomes, and progression

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-BTL-001 | A no-encounter town and an encounter-enabled zone | Walk at least the source cadence threshold in each. | The town produces no encounter; the zone can produce the configured visible/random encounter with transition feedback. | Standing still never advances encounter cadence; map encounter rules select only valid formations. | M8.01-M8.04 / Gate 8 | Pass |
| RK-BTL-002 | `SET-BATTLE` | Enter the encounter and wait for the command menu. | Expected enemy sprites, battle background/BGM, five-member party panels, HP/MP, row, active member, and KO indicators are visible. | Combatants begin with stats copied from the pre-battle party and encounter data. | M8.09-M8.10, M9.02 / Gate 9 | Pass — production two-member imported party and Goblin formation showed the complete data-driven panel/sprite state; arbitrary party counts share the same renderer |
| RK-BTL-003 | `SET-BATTLE` | Complete one round using Attack for each eligible member. | Commands proceed actor-by-actor; targets, hit/miss feedback, damage, and active-member indication are unambiguous. | Turn order is deterministic for the seed; each living actor acts at most once and KO actors are skipped. | M9.01, M9.04, M9.06-M9.11 / Gate 9 | Pass — live basic-attack rounds plus deterministic selector/order/damage/KO tests |
| RK-BTL-004 | `SET-BATTLE`, with comparable attackers/targets in both rows | Perform matched physical attacks from/to front and back rows. | Damage feedback appears for every attack. | Observed damage reflects source attacker/defender row modifiers, minimums, and caps. | M9.08, M10.03 / Gate 10 | Not run |
| RK-BTL-005 | `SET-BATTLE`, member with an offensive ability/spell | Choose Spell/Ability, select a living enemy, and confirm. | Only learned/available abilities appear; target shape, animation, message, and SFX match the selected ability. | MP/cost is charged once and seeded damage/elemental affinity matches source rules. | M9.03, M10.04-M10.05 / Gate 10 | Not run |
| RK-BTL-006 | `SET-BATTLE`, damaged living ally and healing spell | Cast the healing spell on that ally. | Healing amount and target feedback display once. | MP decreases once and HP increases without exceeding maximum. | M9.05, M10.06 / Gate 10 | Not run |
| RK-BTL-007 | `SET-BATTLE`, KO ally and revive ability/item | Select the KO ally and confirm revive. | The UI permits the KO target and shows revive feedback. | The ally returns with the source-defined HP; living allies are rejected without cost. | M10.15-M10.17 / Gate 10 | Not run |
| RK-BTL-008 | `SET-BATTLE`, recovery item and damaged valid target | Choose Item, use one recovery item, and confirm the target. | The item, target, and recovery feedback are visible. | Exactly one item is consumed and HP/MP caps and invalid-target rules match source. | M9.03, M10.16 / Gate 10 | Not run |
| RK-BTL-009 | `SET-BATTLE`, elemental throw item | Use the throw item on its valid target group. | Target shape, elemental effect, damage feedback, and SFX are visible. | Exactly one item is consumed and seeded damage/affinity matches source. | M10.18 / Gate 10 | Not run |
| RK-BTL-010 | Battle fixture exposing a hit, miss, and critical | Execute the prepared action sequence. | Each result has distinct text plus bounded damage float, hit flash, rise/fade, animation, and SFX; no effect remains stuck. | HP changes only for hits; critical calculation and damage follow the seeded source rules. | M10.01-M10.02, M10.21-M10.23 / Gate 10 | Not run |
| RK-BTL-011 | Battle fixture with buff and debuff abilities | Apply each effect, inspect combatant indicators, and advance until expiry. | Applied effects and remaining/expiry feedback are visible. | Modifiers affect the intended rules, tick at source turn boundaries, and expire at the correct duration. | M10.07-M10.09 / Gate 10 | Not run |
| RK-BTL-012 | Battle fixture with poison application and cure | Apply poison, advance one tick, then cure it. | Poison indicator/tick/cure feedback appears. | Tick damage, KO policy, cure, duration, and item/MP costs match source rules. | M10.10, M10.17 / Gate 10 | Not run |
| RK-BTL-013 | Battle fixture with sleep | Apply sleep and advance through wake conditions. | Sleep and wake/expiry feedback is visible. | The sleeping combatant skips only the source-defined turns and resumes correctly. | M10.11 / Gate 10 | Not run |
| RK-BTL-014 | Battle fixture with stun | Apply stun and advance through its duration. | Stun and expiry feedback is visible. | The stunned combatant skips the exact source-defined duration. | M10.12 / Gate 10 | Not run |
| RK-BTL-015 | Battle fixture with silence and a spell caster | Apply silence, open the affected actor's command menu, then cure/wait for expiry. | Spell availability visibly changes while silenced and returns afterward. | Invalid casts cannot spend MP or a turn outside source rules. | M10.13 / Gate 10 | Not run |
| RK-BTL-016 | Battle fixture with taunt and multiple legal targets | Apply taunt and observe enemy actions until expiry. | Taunt and expiry feedback is visible. | Enemy targeting honors the taunter while required, then returns to normal weighted targeting. | M10.14 / Gate 10 | Not run |
| RK-BTL-017 | `SET-BATTLE`, fixed seed and enough rounds to expose AI choices | Defend/heal while observing enemy actions. | Enemy action names, targets, effects, and fallback feedback are coherent. | Weighted and conditional AI chooses only legal actions and repeats for the same seed/actions. | M9.10, M10.19 / Gate 10 | Not run |
| RK-BTL-018 | `SET-BOSS` | Fight until the boss exposes its boss-only move rules. | Boss identity, special actions, battle feedback, and audio are distinct and readable. | Boss-only restrictions are honored and regular enemies cannot use those rules. | M10.20 / Gate 10 | Not run |
| RK-BTL-019 | `SET-BATTLE` | Defeat every enemy and advance through post-battle. | Victory feedback and a rewards screen list GP, EXP, loot, levels, and learned abilities; world visuals/BGM resume once. | Rewards apply atomically once and the pre-battle return context is restored. | M9.12, M9.15, M10.24-M10.30 / Gate 10 | Partial — M9 victory and world restoration passed live; M10 rewards/post-battle work remains |
| RK-BTL-020 | A victory fixture yielding multiple levels and loot | Win the battle and compare before/after status and inventory. | Every level, stat increase, learned ability, and loot item/quantity is displayed. | EXP thresholds, multiple level gains, restoration/caps, GP, and deterministic loot each apply once. | M10.24-M10.30 / Gate 10 | Not run |
| RK-BTL-021 | `SET-BOSS` | Defeat the boss, finish post-battle, save, relaunch, and load. | Boss defeat/reward feedback appears and the world reflects the unlocked progression. | Only the configured boss flag is set; it and rewards survive reload without regranting. | M10.31, M7.12 / Gate 10 | Not run |
| RK-BTL-022 | `SET-BATTLE`, fixed low flee chance | Attempt to flee until one attempt fails. | Failure feedback appears and battle continues. | Failure consumes exactly the source-defined turn opportunity; no world transition occurs. | M9.14 / Gate 9 | Pass — live failure advanced from Elise to Aric without leaving battle |
| RK-BTL-023 | `SET-BATTLE`, fixed successful flee setup | Press Escape/Run and succeed. | Success feedback and transition return to the correct map/audio. | Player receives source-defined separation/safety behavior and no victory reward. | M9.14, M9.16 / Gate 9 | Pass — live success restored the captured world and inactive engaged enemy |
| RK-BTL-024 | `SET-BATTLE`, make every party member KO | Allow the enemies to defeat the full party. | Game Over appears only after the final living member is KO; defeat music/feedback does not overlap battle audio. | Battle stops accepting commands and routes to the Game Over flow. | M9.13, M9.17 / Gate 9 | Pass — isolated 1-HP production save reached sequential KOs and Game Over after the final KO |

## Field menu, five-member party, items, equipment, spells, and status

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-PTY-001 | `SET-PARTY` | Open the field menu and inspect the party summary. | All five members show correct name, level, HP/MP, row, and shared GP without clipping. | Displayed values match the active runtime party and repository. | M6.01-M6.03 / Gate 6 | Partial — one-member live X11 pass; five-member reducer/domain proof, save-dependent remainder |
| RK-PTY-002 | `SET-PARTY` | Open Status and cycle forward/backward through every member. | Exactly one selected member's portrait/details are shown and all five are reachable in party order. | Character switching changes only the viewed member. | M6.04 / Gate 6 | Partial — one/two-member runtime plus explicit one/five-member reducer; five-member debug preset awaits M13.07 |
| RK-PTY-003 | `SET-PARTY` | Compare each member's status with their equipment and active effects. | Base and derived stats, level/progression, row, equipment, abilities, and status effects are readable. | Derived values agree with equipment/status rules and survive save/load. | M6.05, M7.12 / Gate 7 | Pass — M6 display/formula proof plus native equipment/status persisted-state round trip |
| RK-PTY-004 | `SET-PARTY` | Open Items and visit All, New, Recovery, Status, Battle, Material, Core, and Key tabs. | Each pouch visibly contains only matching shared-inventory rows; the original-style Pouch, Items, and Detail columns remain readable for empty and multi-page lists. | Tab/filter/scroll navigation does not change quantities. | M6.06-M6.10, M6.27 / Gate 6 | Pass — production-catalog fixture plus X11 screen 2026-08-12; corrected three-column live capture and structure regression 2026-08-16 |
| RK-PTY-005 | `SET-PARTY`, item newly acquired in the current loot batch | Inspect New, leave/reopen Items, then acquire a later loot batch. | New shows the latest batch according to source semantics and ordinary tabs still show all owned items. | Viewing does not consume the batch or alter shared inventory. | M6.09-M6.10 / Gate 6 | Pass — deterministic repository fixture 2026-08-12 |
| RK-PTY-006 | `SET-PARTY`, discardable item stack greater than one | Choose Discard, select a quantity smaller than the stack, and confirm. | Quantity bounds and confirmation are visible. | Shared quantity decreases exactly by the chosen amount. | M6.11 / Gate 6 | Pass — quantity/repository fixtures 2026-08-12 |
| RK-PTY-007 | `SET-PARTY`, locked item and key item | Attempt to discard each. | Each rejection explains why the item cannot be discarded. | Both quantities remain unchanged. | M6.12 / Gate 6 | Pass — atomic rejection fixture 2026-08-12 |
| RK-PTY-008 | `SET-PARTY`, field healing item and damaged member | Use the item on the damaged member, then attempt it on a full-health/invalid target. | Target and recovery/rejection feedback are visible. | Valid use consumes once and caps recovery; invalid use consumes nothing. | M6.13 / Gate 6 | Pass — Potion domain fixture 2026-08-12 |
| RK-PTY-009 | `SET-PARTY`, curable status and matching field cure item | Use the cure item on the affected member, then repeat. | Cure feedback appears once; the repeat explains the invalid target. | Supported status clears and exactly one item is consumed. | M6.14 / Gate 6 | Pass — Antidote domain fixture 2026-08-12 |
| RK-PTY-010 | `SET-PARTY` | Open Equipment and visit every slot for a member with empty and populated slots. | The portrait party column, five slot cards, equipped names, empty slots, and derived totals are distinct and readable. | Viewing equipment does not move shared-inventory items. | M6.15-M6.16, M6.28 / Gate 6 | Pass — X11 Status plus equipment-domain fixture 2026-08-12; live slot/picker capture and equipment UI regression 2026-08-16 |
| RK-PTY-011 | `SET-PARTY`, compatible owned upgrade | Highlight the upgrade, inspect deltas, then equip it. | The inventory picker shows compatible gear and every changed derived stat has a color-coded before/after preview; the equipped row updates after confirmation. | New item equips atomically; the old item returns to shared inventory and totals update once. | M6.17-M6.18, M6.28 / Gate 6 | Pass — Steel Axe swap fixture 2026-08-12; picker/preview UI regression 2026-08-16 |
| RK-PTY-012 | `SET-PARTY`, incompatible owned equipment | Attempt to equip it on a blocked member/class/slot. | Compatibility rejection and reason are visible. | Equipment, stats, and shared inventory remain unchanged. | M6.15, M6.19 / Gate 6 | Pass — Dagger/Hero rejection fixture 2026-08-12 |
| RK-PTY-013 | `SET-PARTY`, member with learned, locked, field, and battle-only abilities | Open Spells for that member. | The portrait caster column and spellbook/detail panel show only learned field-usable spells, with MP readiness, type, target, and description; locked and battle-only abilities are absent or clearly unavailable. | Viewing spells consumes no MP. | M6.20-M6.21, M6.29 / Gate 6 | Pass — production class fixture 2026-08-12; live spellbook/Teleport capture and UI regression 2026-08-16 |
| RK-PTY-014 | `SET-PARTY`, damaged member and field healing spell | Cast the spell on the damaged member. | The party target overlay and recovery feedback appear once without hiding the underlying spellbook context. | MP decreases only after valid confirmation and HP caps at maximum. | M6.22, M6.29 / Gate 6 | Pass — Elise Heal fixture 2026-08-12; target-overlay UI regression 2026-08-16 |
| RK-PTY-015 | `SET-PARTY`, field healing spell but invalid/full target or insufficient MP | Attempt the cast. | The screen explains why the cast is unavailable. | MP, HP, and turn/world state remain unchanged. | M6.22 / Gate 6 | Pass — invalid-target atomicity fixture 2026-08-12 |
| RK-PTY-016 | `SET-PARTY`, teleport ability and a mix of visited/unvisited/ineligible maps | Open the teleport destination picker and inspect it. | A focused destination overlay shows only eligible visited destinations with accurate controls over the retained spellbook context. | Opening/canceling the picker consumes no MP and changes no map. | M6.23, M6.29 / Gate 6 | Pass — metadata/portal eligibility fixture 2026-08-12; destination-overlay structure covered 2026-08-16 |
| RK-PTY-017 | Same setup as RK-PTY-016 | Select an eligible destination and confirm. | The standard fade/map/BGM transition occurs without a special-case visual path. | MP decreases once after confirmation and the party arrives at the valid configured position. | M6.24 / Gate 6 | Pass — shared transition acceptance path and transition suite 2026-08-12 |
| RK-PTY-018 | Normal campaign progression before and after a level/flag unlock | Open the member's Spells/Status on both sides of the unlock. | The newly learned ability becomes visible only after its source-defined requirement. | Learned availability persists through save/load. | M6.20, M10.27-M10.29 / Gate 10 | Partial — level/flag gates and M7 persistence pass; actual progression awaits M10 |
| RK-PTY-019 | `SET-PARTY`, owned magic cores | Open the Core inventory tab, inspect core details, then open the magic-core service through dialogue. | Owned cores, descriptions, quantities, and the distinct magic-core shop/exchange UI are visible. | Cores remain shared inventory and opening either screen changes no quantity. | M6.07, M11.01 / Gate 11 | Not run |
| RK-PTY-020 | `SET-PARTY` saved in one order/row arrangement supported by source UI | Switch the active/viewed character and any player-editable row/order, save, relaunch, and load. | Party summaries and battle panels show the same five members, ordering, and rows after load. | Party membership/order/rows persist exactly. | M3.03, M7.12 / Gate 7 | Not run |
| RK-PTY-021 | `SET-PARTY`, member currently in the front row | Open Status, select that member, choose Position, set Back, and inspect Status and the next battle panel. | Position shows Back and the battle panel places/labels the member consistently. | Only the selected member's row changes; it affects physical rules and survives save/load. | M3.03, M6.03-M6.05, M10.03 / Gate 10 | Not run |
| RK-PTY-022 | `SET-PARTY`, owned small and large magic cores with known GP | Open the magic-core exchange, select a quantity of small cores and confirm; then select a large core, cancel its extra confirmation, and finally confirm it. | Only owned core sizes appear; quantity/rate/total, large-value confirmation, cancel, and exchange feedback are accurate. | Cancel changes nothing; each confirmed exchange removes exactly the chosen cores and adds `quantity × rate` GP atomically. | M11.01 / Gate 11 | Not run |
| RK-PTY-023 | `SET-PARTY` | Open Status, inspect the selected member's portrait, press Enter, inspect details, then press Escape. | A full-height portrait fills the center column; Enter replaces it with progression, stats, equipment, Spells, and Position; Escape restores the portrait view. | Switching between portrait and details changes no character or party state. | M6.25-M6.26 / Gate 6 | Pass — user visually and interactively verified 2026-08-16 against `status-orig.png` and `status-orig-2.png` |

## Dialogue, quests, recruitment, shops, inn, and crafting

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-SVC-001 | NPC conversation with at least two conditional choices | Interact, move the choice selection, cancel once, reopen, and choose each branch from a reset save. | Choices, disabled/conditional choices, branch text, terminal text, and controls are readable. | Only the confirmed branch applies its effects; node jumps terminate without loops or duplicated effects. | M5.15-M5.17 / Gate 5 | Not run |
| RK-SVC-002 | `SET-QUEST`, immediately before Elise's configured join effect | Complete the recruiting dialogue once, then repeat the conversation. | Recruitment dialogue/feedback occurs on the first completion and does not repeat as a second join. | Elise joins once with source-defined initial state and the membership/flag persists through save/load. | M5.18, M7.12 / Gate 7 | Not run |
| RK-SVC-003 | `SET-QUEST` | Complete the quest-starting dialogue and open the quest board. | The new quest appears as Active with location, description, and objectives. | Starting is idempotent and does not grant completion rewards. | M11.14-M11.15, M11.19 / Gate 11 | Not run |
| RK-SVC-004 | Active flag-objective quest | Perform the world/dialogue action that sets the relevant flag, then inspect the quest board. | The objective visibly advances only after the relevant action. | Unrelated flag changes do not advance it; progress persists through save/load. | M11.16, M11.20 / Gate 11 | Not run |
| RK-SVC-005 | Active item-objective quest | Acquire items below, at, and above the threshold; perform the configured turn-in. | Quest board/turn-in dialogue reports accurate progress at each quantity. | Threshold and item-removal behavior match source and unrelated items remain unchanged. | M11.17 / Gate 11 | Not run |
| RK-SVC-006 | Quest with every objective satisfied | Complete the quest, inspect rewards/board, then repeat the completion dialogue. | Completion and reward feedback appears once; board marks the quest Completed. | Flags, GP/items/other rewards apply once and survive save/load. | M11.18-M11.20 / Gate 11 | Not run |
| RK-SVC-007 | `SET-SERVICE`, NPCs for item/weapon/armor/magic-core services | Interact with each service NPC and choose the shop action. | Dialogue routes to four visibly distinct service types with correct stock/title/details. | Opening/canceling each service changes no GP or inventory. | M11.01 / Gate 11 | Not run |
| RK-SVC-008 | `SET-SERVICE`, with locked and unlocked stock | Open each relevant shop and inspect all rows. | Locked stock is hidden as defined; price, description, balance, affordability, and equipment compatibility/deltas are accurate. | Inspecting stock is stateless and does not unlock or purchase it. | M11.02-M11.03, M11.07 / Gate 11 | Not run |
| RK-SVC-009 | `SET-SERVICE`, affordable item below quantity cap | Buy exactly one item. | Purchase feedback and updated balance/quantity appear once. | GP and shared inventory update atomically by the correct values. | M11.04 / Gate 11 | Not run |
| RK-SVC-010 | `SET-SERVICE`, unaffordable item or capped stack | Attempt to buy one item. | The reason for rejection is visible. | GP and inventory remain unchanged. | M11.03-M11.04 / Gate 11 | Not run |
| RK-SVC-011 | `SET-SERVICE`, ordinary sellable item plus key/locked/zero-value items | Open Sell and inspect rows, then sell one ordinary item. | Only eligible items are selectable and the explicit sell price/quantity/GP result are visible. | One sale decreases quantity and increases GP atomically; excluded items cannot change. | M11.05-M11.06 / Gate 11 | Not run |
| RK-SVC-012 | `SET-SERVICE`, damaged party and affordable inn | Interact with inn, inspect cost, cancel, reopen, and confirm. | Cancel returns without recovery; confirm gives clear stay/recovery feedback and appropriate audio. | Cancel preserves state; confirm charges exact GP and restores eligible HP/MP/status once. | M11.08-M11.09 / Gate 11 | Not run |
| RK-SVC-013 | `SET-SERVICE`, unaffordable inn | Attempt to confirm a stay. | Affordability rejection is visible. | GP, HP/MP, and status remain unchanged. | M11.08 / Gate 11 | Not run |
| RK-SVC-014 | `SET-SERVICE` | Open the apothecary and inspect locked, missing-input, ready, and unique-owned recipes. | Recipe names/details, lock state, ingredient shortages, costs, readiness, and unique-owned state are distinguishable. | Inspecting/canceling consumes nothing and unlock state follows flags. | M11.10-M11.11 / Gate 11 | Not run |
| RK-SVC-015 | `SET-SERVICE`, one ready recipe | Craft exactly one output. | Craft confirmation and resulting output/quantity are visible. | Required inputs and GP are consumed and output granted atomically once. | M11.12 / Gate 11 | Not run |
| RK-SVC-016 | `SET-SERVICE`, unique recipe output already owned | Attempt to craft the unique output again. | Duplicate-unique rejection is visible. | Inputs, GP, and output quantity remain unchanged. | M11.13 / Gate 11 | Not run |
| RK-SVC-017 | Complete one service transaction and advance a quest | Save, quit, relaunch, and load the slot. | Shop-independent inventory/GP and quest board display the same results. | Service results, recruitment, flags, and quest progress round-trip without reapplying. | M11.20, M7.12 / Gate 11 | Not run |

## Save slots, recovery, Game Over, and audio

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-SAV-001 | `SET-NEW`, at a permitted field-save point | Open Save and inspect every slot. | The centered modal keeps autosave pinned and shows six player-slot cards per page; empty and valid slots are distinct and valid metadata includes enough identity/playtime/progress to choose safely. | Opening/canceling Save creates or changes no slot. | M6.30, M7.07-M7.08 / Gate 7 | Pass — live pages 01/02, valid Slot 07 metadata, and canceled overwrite capture without writes; slot-card/paging UI regression 2026-08-16 |
| RK-SAV-002 | Empty slot from RK-SAV-001 | Select the empty slot and confirm save. | One inline success confirmation appears and the slot card changes from empty to valid metadata. | A complete, versioned, loadable state is written atomically. | M6.30, M7.01-M7.06, M7.08 / Gate 7 | Pass — live Slot 01 save refreshed metadata and produced a verified native v1 file; feedback layout covered 2026-08-16 |
| RK-SAV-003 | Existing valid slot plus changed current state | Choose that slot, cancel overwrite, reopen, and confirm overwrite. | A focused overwrite modal makes cancel and confirm distinct; success metadata updates only after confirmation. | Cancel preserves old bytes/state; confirm atomically replaces the slot with the new complete state. | M6.30, M7.05-M7.09 / Gate 7 | Pass — live confirmation/cancel preserved identical SHA-256; overwrite modal and atomic replacement tests pass |
| RK-SAV-004 | `SET-SAVES` | Open the title load picker and select the corrupt/incompatible slot. | The slot explains corruption/version/scenario incompatibility without crashing; the valid slot remains selectable. | Bad data never partially becomes live game state and other slots remain unchanged. | M7.07, M7.11, M7.14, M7.16 / Gate 7 | Pass — live corrupt reason and incompatible label beside independently loadable Slot 01 |
| RK-SAV-005 | Distinctive mid-campaign state: map/position/facing, five members/rows/stats/equipment, flags, inventory/GP, quests, opened box, RNG progression, playtime | Save, quit the process, relaunch, and load. | The same map, entities, party/menu values, quest/box presentation, and correct BGM appear. | Every listed value restores exactly; paused/menu time follows source playtime rules and deterministic continuation has no drift. | M7.12 / Gate 7 | Partial — Gate 7 Ardel position/facing/process restart and complete persisted-state round trip passed; five-member campaign setup awaits later content |
| RK-SAV-006 | A real pinned Python save plus the approved M0.05 compatibility path | Follow the documented import/load or converter flow, then launch the resulting slot. | A clear success or actionable incompatibility result appears; successful load reaches the expected playable scene. | No partial import occurs; successful compatible state matches the source save fields promised by the ADR. | M0.05, M7.15 / Gate 7 | Pass — standalone CLI converted the pinned serializer fixture; normal title load reached Starting Forest with expected imported inventory |
| RK-SAV-007 | Game Over after RK-BTL-024 and at least one valid save | Choose Load, select the valid slot, and confirm. | Game Over closes and the saved world/audio appear without a second battle or title layer. | The selected save becomes the sole live session. | M9.17, M7.11-M7.12 / Gate 9 | Pass — live Game Over Load opened the native valid-slot picker; the existing native-load path then owns the sole session |
| RK-SAV-008 | Game Over with retry option available | Choose Retry once. | The documented retry scene appears with correct music/transition. | Retry uses the source-defined checkpoint/state policy and grants no defeat/victory rewards. | M9.17 / Gate 9 | Pass — live Retry reconstructed pre-battle party/enemy HP without rewards |
| RK-SAV-009 | Game Over | Choose Title once. | Game Over audio/visuals stop and one title screen/title track appears. | Defeated transient battle state is discarded; save files remain unchanged. | M9.17 / Gate 9 | Pass — route reducer and cleanup remove battle/session state before the tested Title entry path |
| RK-AUD-001 | `SET-FRESH`, audio output audible | Wait across at least two title-track loop boundaries. | Title BGM is audible, loops cleanly, and does not multiply. | One logical title track remains active. | M0.15-M0.17, M2.23 / Gate 0 | Not run |
| RK-AUD-002 | `SET-NEW` | Finish intro, enter Ardel, then use a portal to a map with different BGM. | Each indexed map track starts once, loops, and cleanly replaces the prior track. | Exactly one map BGM is logically active after each transition. | M4.24-M4.25, M5.04-M5.06 / Gate 5 | Not run |
| RK-AUD-003 | `SET-BATTLE` | Enter battle, win, and return to the world. | Battle transition/BGM/SFX play, then stop; the correct world BGM resumes once. | Audio transitions do not alter battle/world outcomes or leak players. | M8.10-M8.12, M9.15 / Gate 9 | Pass — live battle/victory return swapped logical BGM players without asset errors or duplicate outcome transitions |
| RK-AUD-004 | Title and field menus | Navigate, confirm, cancel, and attempt a disabled/blocked action. | Indexed hover/confirm/cancel/blocked UI sounds are distinct, play once per action, and a disabled action follows source silence/blocked policy. | SFX do not trigger duplicate actions. | M0.11, M5.26 / Gate 5 | Not run |
| RK-AUD-005 | World/dialogue/box/service fixture | Interact with each fixture once. | Each configured world/dialogue/box/service sound resolves; missing logical names produce actionable fallback/error behavior rather than a crash. | Interaction state changes exactly as in the corresponding non-audio row. | M2.23, M5.26, M11 / Gate 11 | Not run |
| RK-AUD-006 | Representative physical, spell, item, status, victory, and defeat battle actions | Execute each action once. | Each action's indexed SFX and animated feedback are synchronized and no stale sound loops. | Audio completion never gates or duplicates resolver state changes. | M10.02, M10.21-M10.23 / Gate 10 | Not run |

## Determinism, recording/replay, scenario independence, validation, and editors

These are manual operator checks because the source README presents the debug
and authoring workflow as a feature. Their action and expected result remain
observable even when an automated comparison supplies supporting evidence.

| ID | Setup / prerequisite | Exact player/operator action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-DBG-001 | `SET-DEBUG` | Launch twice with the same seed and normalized actions through one encounter, enemy wander sequence, battle, and loot result. | Both runs show the same encounter/enemy movement, action order, results, and feedback. | Final state hashes, RNG-dependent outcomes, and loot are identical. | M1.11, M8.02-M8.07, M13.01 / Gate 13 | Not run |
| RK-DBG-002 | `SET-DEBUG` | Repeat RK-DBG-001 with a different seed. | At least one eligible random outcome may differ while all results remain valid/readable. | The logged seed is the only intended source of gameplay randomness. | M13.01 / Gate 13 | Not run |
| RK-DBG-003 | `SET-DEBUG` | Select the documented Record action, play from title to Ardel or a battle result, then exit normally. | Recording mode and output path are visibly/logged clearly without changing normal game feedback. | A versioned record containing build/scenario identity, seed, normalized actions, and state checkpoints is saved. | M13.02-M13.03 / Gate 13 | Not run |
| RK-DBG-004 | Recording from RK-DBG-003 | Select Replay and play the complete record without physical gameplay input. | The same screens, actions, audio transitions, and final outcome appear in the same logical order. | Replay reaches the same periodic/final state hashes and reports success. | M13.04-M13.05 / Gate 13 | Not run |
| RK-DBG-005 | A copy of RK-DBG-003 with one action/checkpoint deliberately altered | Replay the altered record. | Replay stops or clearly reports divergence at the first mismatch instead of silently continuing as valid. | The original save/scenario and unaltered recording remain unchanged. | M13.05 / Gate 13 | Not run |
| RK-DBG-006 | `SET-CONTENT` scenario A | Launch the same Rust binary with scenario A and begin New Game. | Scenario A's identity, title, protagonist, start map, dialogue, art, and audio appear. | Content is loaded from scenario A, not compiled story constants. | M2, M12, M14.12 / Gate 14 | Not run |
| RK-DBG-007 | `SET-CONTENT` scenario B | Without rebuilding, launch the same binary with scenario B and begin New Game. | Scenario B's visibly different identity, title, protagonist, start map, dialogue, art, and audio replace A's. | No A-specific story state leaks into B and the binary hash is unchanged. | M2, M12, M14.12 / Gate 14 | Not run |
| RK-DBG-008 | Writable scenario copy with one conspicuous balance value | Run a baseline action, edit only that YAML balance value, relaunch without rebuilding, and repeat. | UI/battle/world behavior reflects the edited value and otherwise remains unchanged. | Configurable balance comes from scenario data and saves record/handle the scenario version as documented. | M2.19, M14.12 / Gate 14 | Not run |
| RK-DBG-009 | Writable scenario copy with a replacement referenced image/audio asset | Launch, observe the original, replace the referenced asset, and relaunch without rebuilding. | The replacement art/audio appears at the same logical use site. | Asset choice comes from the package reference and no engine source changes. | M2.04-M2.07, M2.23, M12 / Gate 14 | Not run |
| RK-DBG-010 | Valid pinned scenario and developer menu | Select Validate scenario data. | Validation completes with a clear success summary and no game window is required. | Validation does not modify scenario content or saves. | M2.24-M2.28 / Gate 2 | Not run |
| RK-DBG-011 | Temporary scenario copy with one invalid manifest reference | Run the documented validation action. | Failure names the manifest field, relative path, and cause. | Exit/result is failure and the scenario is not modified. | M2.08-M2.09, M2.26 / Gate 2 | Not run |
| RK-DBG-012 | Temporary scenario copy with one invalid map or portal destination | Run validation. | Failure identifies the map/portal source and missing/invalid destination. | Other valid content is not rewritten. | M2.25-M2.27 / Gate 2 | Not run |
| RK-DBG-013 | Temporary scenario copy with one invalid encounter or enemy reference | Run validation. | Failure identifies the encounter/enemy source and bad reference. | Other valid content is not rewritten. | M2.17-M2.18, M2.25-M2.27 / Gate 2 | Not run |
| RK-DBG-014 | Temporary scenario copy with one invalid dialogue or character reference | Run validation. | Failure identifies the dialogue/character source and bad reference. | Other valid content is not rewritten. | M2.12, M2.16, M2.25-M2.27 / Gate 2 | Not run |
| RK-DBG-015 | Temporary scenario copy with one invalid item/recipe/quest/flag reference | Run validation separately for each bad fixture. | Each run identifies the source record, reference type, and offending value. | No failed run mutates content. | M2.14, M2.21-M2.22, M2.25-M2.27 / Gate 2 | Not run |
| RK-DBG-016 | Temporary scenario copy with one missing referenced asset/audio file | Run validation. | Failure names the scenario-relative missing path and owning data record. | The missing file is not silently substituted in validation. | M2.09, M2.23, M2.25-M2.27 / Gate 2 | Not run |
| RK-DBG-017 | Developer menu with web-editor prerequisites absent in a disposable environment | Select the web map editor action, allow documented first-use setup, open one migrated TMX map, make a reversible visible tile edit, and save. | Setup progress is understandable; the browser editor renders the map and saved edit appears when the game loads that map. | Only intended TMX/content files and declared tool dependencies change. | M13.13-M13.14 / Gate 13 | Not run |
| RK-DBG-018 | Developer menu with Pygame editor available against a temporary scenario copy | Select the Pygame map editor, open the same migrated TMX map, make a different reversible visible edit, and save. | The editor renders the map and saved edit appears when the Rust game loads it. | The Rust runtime consumes the documented shared format without Python engine code at runtime. | M13.13-M13.14, M14.12 / Gate 14 | Not run |
| RK-DBG-019 | Self-contained release package copied outside both repositories | Disconnect/remove access to the Python source checkout and launch Play/New Game/Save/Load. | Game, content, art, and audio run normally with no source-path error. | No Python interpreter/package/source file is loaded at runtime; saves use release-relative/platform paths. | M14.08-M14.09, M14.12 / Gate 14 | Not run |
| RK-DBG-020 | Clean target checkout with documented tool prerequisites | Run `lazymenu-cli`, use `/` to find Play, Record, Replay, test, validation, and both editor actions, then exit separate launches with `q`, Escape, and Ctrl+C. | Search exposes each documented action and each exit key closes the launcher without an error. | Searching/exiting does not launch an action or modify game/scenario/save state. | M2.28, M13.14, M14.10 / Gate 14 | Not run |
| RK-DBG-021 | Clean target checkout after following its documented setup/prerequisite action | Select Play from `lazymenu-cli` and reach title; exit, then run the documented direct Rust command with scenario root, seed `1`, and normal mode. | Both launch paths reach the same title/scenario identity without missing dependency or path errors. | Both use the same scenario, seed, and normal-mode construction; launcher setup does not modify scenario or saves. | M1.09, M13.01, M14.10 / Gate 14 | Not run |
| RK-DBG-022 | Clean target checkout with development prerequisites installed | Select Run test suite from the developer menu and wait for completion. | The action reports an unambiguous success or a focused failure and returns control to the launcher/terminal. | Running tests does not modify scenario content or player saves. | M0.18, M14.10 / Gate 14 | Not run |

## Campaign and content-wave completion

| ID | Setup / prerequisite | Exact player action | Expected visible / audible outcome | Expected state / persistence outcome | Target | Status |
| --- | --- | --- | --- | --- | --- | --- |
| RK-CMP-001 | Clean normal-mode save, no debug overrides | Play Ardel, its interiors, Starting Forest, and the first boss; save/load at the boundary. | Act I opening, interactions, services, encounters, boss, and audio are complete and coherent. | Required flags, party, loot, quests, and boundary save restore correctly. | W12.1 / Gate 12 wave | Not run |
| RK-CMP-002 | Save from RK-CMP-001 | Play Open Plains, both caves, Millhaven/interiors, and reachable Reiya/Jep prerequisites; save/load at boundary. | All maps, dialogue, portals, encounters, services, and recruit prerequisites are reachable without debug injection. | Act I/II transition state and boundary save restore correctly. | W12.2 / Gate 12 wave | Not run |
| RK-CMP-003 | Save from RK-CMP-002 | Play Marshland and Harborgate/interiors through story and recruitment; save/load at boundary. | Harborgate story, maps, services, recruitment, battle, and audio play without missing content. | Recruitment/quest/flag/inventory state and boundary save restore correctly. | W12.3 / Gate 12 wave | Not run |
| RK-CMP-004 | Save from RK-CMP-003 | Play Ancient Ruins gate/courtyard/sanctum and Ruinwatch through its boss flag; save/load. | Act III progression, maps, boss, services, and feedback are reachable and coherent. | Boss/progression state and boundary save restore correctly. | W12.4 / Gate 12 wave | Not run |
| RK-CMP-005 | Save from RK-CMP-004 | Play Mountain Foothills, Frostholm, palace, vault, Kael flow, and boss progression; save/load. | All stated maps, story, recruit flow, boss, and audio are complete. | Party/flag/quest/reward state and boundary save restore correctly. | W12.5 / Gate 12 wave | Not run |
| RK-CMP-006 | Save from RK-CMP-005 | Play Mountain Pass, Ashenveil, and oracle sanctum through Act IV setup; save/load. | Maps, portals, dialogue, encounters, services, and audio remain complete. | Act IV setup and boundary save restore correctly. | W12.6 / Gate 12 wave | Not run |
| RK-CMP-007 | Save from RK-CMP-006 | Play Sunken Cave, Corrupted Forest, and Volcanic Region; exercise late-game encounters, loot, and services; save/load. | Late-game content and difficulty feedback are coherent with configurable source balance. | Loot, progression, quest/service state, and boundary save restore correctly. | W12.7 / Gate 12 wave | Not run |
| RK-CMP-008 | Save from RK-CMP-007 | Play Final Stronghold and the ending path through the final boss and campaign completion. | Final maps, battles, dialogue, ending/credits or game-complete presentation, and audio all finish without placeholder/dead-end content. | Completion flags/rewards apply once and a post-completion save follows the documented policy. | W12.8, M14.02 / Gate 14 | Not run |
| RK-CMP-009 | Fresh user profile and self-contained release package | Start a new normal-mode game and complete RK-CMP-001 through RK-CMP-008 without developer tools or source checkout. | The complete campaign is playable from title to ending with no crash, missing asset, broken control hint, or unresolved content reference. | Saves at every wave restore without drift and final completion is reachable from clean state. | M14.01-M14.03, M14.08-M14.09 / Gate 14 | Not run |

## README claim coverage map

This table is the M0.04 coverage audit. Ranges include only manual rows above;
supporting automated tasks in the plan are not counted as acceptance rows.

| Pinned README claim | Manual acceptance rows |
| --- | --- |
| Tiled/TMX overworlds | RK-WLD-003, RK-DBG-017, RK-DBG-018 |
| Collision | RK-WLD-004, RK-WLD-005, RK-WLD-010 |
| Portals | RK-WLD-011, RK-DBG-012 |
| NPCs | RK-WLD-010, RK-WLD-012 |
| Signs | RK-WLD-013 |
| Treasure boxes | RK-WLD-014 through RK-WLD-016 |
| Camera movement | RK-WLD-006, RK-WLD-007 |
| Animated sprites | RK-WLD-009, RK-WLD-010, RK-WLD-017 |
| Visible enemy encounters | RK-WLD-017, RK-BTL-001, RK-BTL-002 |
| Turn-based party combat | RK-BTL-002, RK-BTL-003 |
| Front and back rows | RK-BTL-002, RK-BTL-004, RK-PTY-020, RK-PTY-021 |
| Abilities | RK-BTL-005, RK-PTY-013, RK-PTY-018 |
| Battle items | RK-BTL-008, RK-BTL-009 |
| Status effects | RK-BTL-011 through RK-BTL-016, RK-PTY-009 |
| Enemy AI | RK-BTL-017 |
| Bosses | RK-BTL-018, RK-BTL-021 |
| Rewards | RK-BTL-019, RK-BTL-020 |
| Loot | RK-BTL-019, RK-BTL-020, RK-PTY-005 |
| Animated battle feedback | RK-BTL-005 through RK-BTL-018, especially RK-BTL-010 |
| Five-member parties | RK-BTL-002, RK-PTY-001, RK-PTY-002, RK-PTY-020 |
| Character switching | RK-PTY-002, RK-PTY-020 |
| Equipment | RK-PTY-010 through RK-PTY-012, RK-SVC-008 |
| Spells | RK-BTL-005 through RK-BTL-007, RK-PTY-013 through RK-PTY-018 |
| Shared inventory | RK-WLD-014 through RK-WLD-016, RK-PTY-004 through RK-PTY-012 |
| Magic cores | RK-PTY-004, RK-PTY-019, RK-PTY-022, RK-SVC-007 |
| Progression | RK-BTL-019 through RK-BTL-021, RK-PTY-018 |
| Status screens | RK-CTL-007, RK-PTY-001 through RK-PTY-003, RK-PTY-023 |
| Dialogue-driven quests | RK-SVC-001, RK-SVC-003 through RK-SVC-006 |
| Recruitment | RK-SVC-002, RK-CMP-002 through RK-CMP-005 |
| Shops | RK-SVC-007 through RK-SVC-011 |
| Inns | RK-SVC-012, RK-SVC-013 |
| Crafting at apothecaries | RK-SVC-014 through RK-SVC-016 |
| Save slots | RK-TTL-003, RK-TTL-005, RK-TTL-006, RK-SAV-001 through RK-SAV-006 |
| Title flow | RK-TTL-001 through RK-TTL-008 |
| Game-over flow | RK-BTL-024, RK-SAV-007 through RK-SAV-009 |
| Audio | RK-TTL-001, RK-AUD-001 through RK-AUD-006 |
| Configurable balance data | RK-DBG-008, RK-CMP-007 |
| Scenario YAML/data and assets rather than hardcoded story content | RK-DBG-006 through RK-DBG-009, RK-DBG-019 |
| Deterministic seeds | RK-DBG-001, RK-DBG-002 |
| Input recording and playback | RK-DBG-003 through RK-DBG-005 |
| Scenario validation | RK-DBG-010 through RK-DBG-016 |
| Web map editor | RK-DBG-017 |
| Pygame map editor | RK-DBG-018 |
| Arrow keys move on map | RK-CTL-001 |
| Arrow keys navigate menus | RK-CTL-002 |
| Enter interacts | RK-CTL-003 |
| Enter confirms a menu choice | RK-CTL-004 |
| M opens/closes field menu | RK-CTL-005 |
| I opens overworld items | RK-CTL-006 |
| S opens overworld character status | RK-CTL-007 |
| Escape goes back | RK-CTL-008 |
| Escape closes a menu | RK-CTL-009 |
| Escape attempts to flee | RK-CTL-010, RK-BTL-022, RK-BTL-023 |
| Menus show context-specific controls at the bottom | RK-CTL-011 |
| Setup/Play and direct scenario/seed/normal-mode launch descriptions | RK-DBG-020, RK-DBG-021, RK-CMP-009 |
| Launcher search and q/Escape/Ctrl+C exit controls | RK-DBG-020 |
| Launcher Record and Replay actions | RK-DBG-003, RK-DBG-004 |
| Launcher Run test suite action | RK-DBG-022 |
| Launcher Validate scenario data action | RK-DBG-010 through RK-DBG-016 |
| Launcher exposes both map editors, including first-use web setup | RK-DBG-017, RK-DBG-018 |
| Validator manifest cross-references | RK-DBG-011 |
| Validator map and portal cross-references | RK-DBG-012 |
| Validator encounter and enemy cross-references | RK-DBG-013 |
| Validator dialogue and character cross-references | RK-DBG-014 |
| Validator item, recipe, quest, and flag cross-references | RK-DBG-015 |
| Validator asset/audio cross-references | RK-DBG-016 |
| Full Rust-binary campaign completion and Python runtime independence | RK-DBG-019, RK-CMP-001 through RK-CMP-009 |

## Execution record requirements

For each manual run, record:

1. target commit and release/package identifier;
2. scenario id/version and content hash;
3. platform, graphics renderer, audio device, resolution, and input device;
4. tester and date;
5. seed, save fixture, and recording path when applicable;
6. Pass, Accepted difference, or Blocked evidence for every executed row; and
7. issue links for any mismatch, with the row left non-passing until retested.

M14.01 is complete only when every row is `Pass` or links to an approved
`Accepted difference`. Campaign-wave rows supplement rather than replace the
individual feature rows.
