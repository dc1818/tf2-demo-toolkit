# Enabled crossbow saves — A04

Activation authorized by owner: "okay enable it" (November 5, 2026).
This change is confined to work/demo-toolkit-roadmap. It is not a main-branch
merge or a published beta release.

## Product behavior

Normal parse/analyze now adds independent, zero-kill candidates tagged
crossbow_save when the observed-damage counterfactual is supported.

- The healer must be an observed Medic on the patient's team.
- Honor the existing live-round and POV-player analysis scope.
- Use the crossbow_heal event, pairing its generic player_healed duplicate
  in the same packet without counting the heal twice.
- Cap attempted healing at missing health. Valve's SDK event amount is
  flHealth, not the returned iActualHealed.
- Require fresh alive/in-PVS snapshots with known protection fields.
- Replay actual damage for at most one second after the arrow. It qualifies
  when the no-arrow health balance reaches zero while actual health stays
  positive, followed by a matching post-event alive snapshot.
- Other healing, unexplained health changes, unknown/changed protection,
  life/identity/round boundaries, stale samples or inconsistent damage math
  cancel attribution. This conservative first version does not try to
  reconstruct ambiguous simultaneous healing or regeneration.
- Freshness tolerance is 250 ms. Server tick interval comes from the new
  additive manifest field server_tick_interval; no guessed clock is used.
- The candidate has score 30, zero kills, explicit counterfactual metrics,
  a description and per-tick evidence. It does not modify any existing
  candidate, score rule, bookmark association, tag or UI layout.
- Both arrow and decisive-damage demo ticks are supplied as recording cues
  through the existing non-kill cue mechanism. This preserves the whole
  event when the user chooses zero trailing padding.

Existing imported candidate exports are not silently reanalyzed. Older
parsed exports remain loadable with their original features; exports lacking
the new time/protection evidence do not gain an unverified save. Parse again
with the updated exporter to obtain the new evidence.

This is a bounded same-observed-damage counterfactual, not a claim that all
players would have acted identically in a different game state.

## Tests

Actual Rust analyzer source, compiled with Rust 1.88 through a temporary
headless harness: 29 analyzer tests passed, including 16 new detector and
integration tests. The harness uses the real analyzer/models/scheduler;
its unused config-path shim does not exercise settings or UI code.

Coverage includes projectile and two-tick afterburn saves; heal caps;
nonlethal damage; duplicate heal/arrow events; stale/unknown protection;
other healing and unexplained HP; death, identity and round boundaries;
POV scope; timing/order; overlapping arrows; missing server clock; and
normal analyzer output preserving an existing bookmark.

The full local GUI build was blocked by missing native pkg-config/fontconfig
development dependencies. This is separate from the headless source tests;
do not claim local GUI compilation or Windows recording verification.

## Real demos

Public fixtures: demostf/parser commit
e1425a7cf72e1656fa8cceea14673f56b59875e2. Fresh exports used the updated
native exporter. Existing-candidate parity was compared using the same
analyzer, first without the optional interval (legacy/no detector), then with
the detector active.

| Demo | Crossbow-heal events | Existing candidates preserved | New saves |
| --- | ---: | ---: | ---: |
| comp (STV) | 28 | 56 | 1 |
| gully (POV) | 0 | 11 | 0 |
| short-2024 | 0 | 0 | 0 |

The positive comp example:
arrow demo tick 45680 / server tick 72818, healer 78, patient 70.
Pre-arrow HP 34 + 58 effective healing = 92.
At demo tick 45704 / server tick 72840, actual recorded damage 39 leaves
53 HP; no-arrow balance 34 - 39 = -5. This is 22 server ticks × 0.015 s,
or 0.33 seconds after the arrow. A post-event state sample reconciles 53 HP.
The generic heal and crossbow event represent the same arrow and are paired.

This is observed-data validation, not independent visual playback labeling.
Afterburn coverage here is synthetic; the sampled real positive used a
recorded damage event without guessing its weapon subtype.

## Sources

[Valve SDK healing-bolt implementation](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_projectile_arrow.cpp):
ImpactTeamPlayer applies TakeHealth and emits generic and crossbow events
with the attempted flHealth. User IDs are used for healer/target.
