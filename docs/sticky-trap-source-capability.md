# A06 sticky-trap source capability

November 26, 2026. Phase 2 evidence checkpoint at pilot revision `e10b5d1`.
Ralph reviewed projectile state, entity identity, removal semantics, event
ordering, and analyzer matching. The measurements cover the two existing
SourceTV demos and one player POV demo from the A13 corpus.

## SourceTV and POV measurements

| Observation | 1530633 SourceTV | comp SourceTV | gully POV |
| --- | ---: | ---: | ---: |
| Recorded remote-sticky deaths with distinct attacker and victim | 20 | 16 | 18 |
| Deaths in the analyzer's live-round and capture scope | 20 | 16 | 0 |
| Deaths with a positive inflictor index | 20 | 16 | 18 |
| Exact inflictor index meets a tracked sticky at the death packet | 20 | 16 | 9 |
| Tracked sticky's original launcher matches only the credited attacker's sampled handles | 20 | 16 | 9 |
| Tracked sticky also disappears from state in the death packet | 0 | 0 | 1 |
| Death and matching player's `demoman_det_stickies` in the same packet | 9 | 9 | 0 |
| Death and one exported explosion effect with the same weapon ID and definition in the same packet | 9 | 9 | 0 |
| Death with both of those same-packet signals | 1 | 5 | 0 |
| Projectiles observed as sticky across distinct tracked spans | 966 | 624 | 609 |
| Entity indices with multiple observed sticky spans | 84 | 131 | 115 |
| Raw `m_bTouched` 0 / 1 and a sticky state update for the same index and packet | 88 / 932 | 45 / 574 | 211 / 559 |
| `demoman_det_stickies` events (any player) | 1,905 | 784 | 0 |
| `player_destroyed_pipebomb` events (any player) | 32 | 37 | 10 |

All three headers report demo format 3 and network protocol 24. Each
manifest records a nominal 15 ms server tick interval. The corpus compares
capture modes and different recordings within that protocol. Coverage of
other protocols and a local-Demoman POV with a reviewed trap kill remains
open. No two measured
sticky deaths from the same attacker shared a packet in these fixtures.

## Capability matrix

| Evidence | Current export and fixture result | What A06 needs |
| --- | --- | --- |
| Sticky type | Normalized state has `projectile_type`. The parser uses `m_iType`; its normalized `sticky` combines damaging stickies and Sticky Jumper projectiles. A death can independently report `tf_projectile_pipe_remote` and a definition index. | Retain the raw pipe mode and its provenance. Resolve entry baselines and later deltas. Keep jumper, loose cannon, and unobserved or unsupported modes distinct. |
| Projectile lifetime | State samples carry entity index, packet sequence, tick namespace, position, launcher, and team. Indexes recur across observed spans in all three demos. | Retain the entry serial or a parser lifetime ID, with message ordering and explicit entry, leave, delete, removal, and round-boundary provenance. Treat entry into visibility as first observation unless the source establishes a new lifetime. |
| Owner at planting and detonation | All 45 tracked death joins have a unique original-launcher match with the credited attacker. That match says whose sampled weapon list held the launcher at death. Raw pipe-class updates carried explicit `m_hThrower` in all three demos (720, 513, 2,280). `m_hOwnerEntity` was declared but had no explicit pipe-class updates in this scan. `comp` supplied 47 explicit `m_hDeflectOwner` updates on the pipe class across all modes. | Track thrower, current and original launcher, and available ownership or deflection changes independently. Resolve full handles to the player identity valid at that packet. Mark ownership unknown across unresolved changes. |
| Planting and dwell | Normalized state has position deltas and no planted flag. Both raw touched values were co-observed with sticky updates in all three demos. The SourceTV observations from first tracked sample to death spanned 54–2,682 and 53–6,979 server ticks respectively. Those intervals can include flight or start after the actual launch. | Keep the observed `m_bTouched` state per lifetime. An observed false-to-true transition bounds planting between samples. First sight already touched can support a lower bound over uninterrupted observation. Reset or qualify the interval on touched reset, movement, or a visibility gap. |
| Detonation vs. destruction | `demoman_det_stickies` carries a player entity index. `player_destroyed_pipebomb` carries a user ID without a projectile index. Neither fixture type supplies bomb IDs in those events. The optional stream named `rocket_explosions.ndjson` contains 364, 274, and 317 effect rows with weapon ID 35. The effect records identify weapon, position, and contact evidence. A state removal can also be a visibility or lifecycle change. | Keep trigger, explosion effect, and disappearance as distinct observations. Resolve the trigger actor in time. Require independent evidence before joining several bombs into one detonation cluster. Preserve ambiguity when effects are absent, duplicated, or shared by plausible bombs. |
| Victims and impact | The death event provides the credited attacker, victim, and inflictor index. The index-level join worked in 45 of 54 recorded sticky deaths. No fixture establishes a multi-victim cluster or a local POV positive. | Join casualties to a particular lifetime with packet and round ordering. Evaluate medic picks and team advantage from separate roster evidence. Test same-bomb and multi-bomb casualties with labeled examples. |

Valve's published game code transfers `m_bTouched` on pipebombs. A sticky
sets the flag when it sticks to a surface and can clear it when dislodged.
The detonation event reports the acting player, and the explosion effect
can carry a contacted entity. Those semantics explain the proposed evidence
rules. The fixture counts above measure what each recording actually
contains. [Pipebomb state and collision](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_weapon_grenade_pipebomb.cpp),
[launcher detonation event](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_weapon_pipebomblauncher.cpp#L396),
[grenade explosion effect](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_weaponbase_grenadeproj.cpp#L320)

## Method and interpretation

- Reused the A13 native packet, event, state-delta, and explosion-effect
  outputs. Verified the event and state stream SHA-256 hashes against the
  [A13 machine-readable baseline](weapon-attribution-baseline.json). Used
  the final A13 no-schema death traces to count analyzer scope.
- Counted raw `player_death` events with exact weapon
  `tf_projectile_pipe_remote` and positive, distinct attacker/victim IDs.
  The state audit replayed deltas in packet order. For the exact inflictor
  join it checked the tracked sticky immediately before or after that
  packet, preferring the prior sample when available and dropping a
  visible same-packet span conflict. Those joins use the current
  index-keyed export and remain provisional until serial-aware.
- Counted explicit pipe-class updates in `packets.ndjson`, with each
  property's identifier resolved from that fixture's data tables. Joined
  touched values to derived sticky delta rows by packet and entity index.
  The raw count covers explicit updates. Entry state can inherit baseline
  values through `PacketEntity::props`, so missing explicit fields do not
  establish absence from the decoded lifetime.
- Matched `demoman_det_stickies.player` to the player entity index found
  through the unique attacker-launcher join. The effect comparison
  required the death's weapon ID and definition index in the same packet.
  Neither comparison establishes which bomb produced the observation.
  The existing explosion writer can skip ambiguous or undecodable
  packets, so its counts describe the available effect stream.

[1530633 demo](https://demos.tf/1530633),
[comp fixture](https://github.com/demostf/parser/blob/e1425a7cf72e1656fa8cceea14673f56b59875e2/test_data/comp.dem),
[gully fixture](https://github.com/demostf/parser/blob/e1425a7cf72e1656fa8cceea14673f56b59875e2/test_data/gully.dem)

## Recommended A06 next slice

Prototype an optional parser evidence export before proposing a detector.
Start with raw type, touched, owner and launcher observations, full
lifetime identity, and packet/message ordering. Keep the original event,
state, and candidate streams available for parity checks. Replay these
three demos and add a reviewed Demoman POV positive and a labeled
multi-victim example. Report source-specific join rates, planted dwell
bounds, and the reasons ambiguous cases remain unresolved.

Include controls for sticky spam and midair detonation, an already-planted
bomb entering visibility, dislodging and replanting, owner changes, index
reuse, destruction and round cleanup, independent same-time detonations,
and a planted group with no kill. Record both positive and negative
fixture evidence before proposing tag or scoring thresholds. The
capability checkpoint leaves the A06 detector at Missing.
