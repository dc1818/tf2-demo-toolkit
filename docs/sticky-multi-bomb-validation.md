# A06 multi-bomb grouping validation

November 27, 2026. Ralph reviewed the evidence available at pilot
revision `00f3ed5`. This checkpoint validates multi-projectile
observations for A06 and records the remaining common-activation gap.

## Labeled fixture results

| Fixture | Recorded packet | Verified association | Limit |
| --- | --- | --- | --- |
| habib i63 POV, planted triple | Demo tick 105576, server tick 156666, sequence 245151 | User 80 killed users 70 and 74 with entity 633 serial 83, and user 73 with entity 636 serial 154. The two projectiles were live at their death messages and resolved to the same thrower, player entity 12 serial 340, and full current/original launcher handle 605318. | The POV contains a nearby secondary-fire input but lacks a server activation-to-bombs record. |
| habib i63 POV, likely air detonation | Demo tick 154193, server tick 200208, sequence 350532 | Users 70 and 73 both died to entity 1190 serial 366. The packet carries one corresponding explosion effect. | The last pre-death position was still changing and touched had never become true. Planted dwell is unknown. |
| 1530633 SourceTV, independent same-tick blasts | Server tick 26930, sequence 1874 | Two distinct effect origins uniquely match entities 140/617 and 436/937. They resolve to different throwers and launchers (users 4 and 5). The packet includes detonation markers for both actors. | Five marker rows accompany two effects. Marker count does not measure the number of explosions. |
| comp SourceTV, co-timed effects from two owners | Server tick 96921, sequence 69757 | Seven complete effect origins uniquely match seven visible mode-1 pipebombs. Six resolve to user 62 and launcher 457761, one to user 76 and launcher 123984. Both actors have detonation markers. | There are no casualty joins in this packet. Effect coincidence is evidence for an owner-specific grouping only. |

The planted POV triple is two lethal projectiles with three distinct
victims. The earlier [POV audit](sticky-evidence-export.md#labeled-demoman-pov-cases)
reported the first two victims under a stricter check that required raw
pipe mode 1. In the expanded audit, entity 636 has an earlier,
separately deleted serial 153. Serial 154 is present and planted at the
third death, and the raw death event attributes that death to remote
sticky damage from index 636. The mode field for serial 154 was
unobserved. Retaining that provenance supports the death association
without treating a previous lifetime's mode as evidence.

The planted lower bounds are **595 ticks (8.925 s)** for 633/83 and
**824 ticks (12.360 s)** for 636/154. The first projectile's explosion
effect has a complete origin within four map units of its last
position. A separate effect has explicit horizontal coordinates
matching 636/154, with height and weapon fields omitted from that
record. That effect is additional corroboration. The packet contains
five explosion records in total. One complete origin has two plausible
bombs, and one record lacks an explicit horizontal coordinate.

The POV's local command at recorded command tick 156664 carries the
secondary-fire bit two ticks before the recorded deaths. The observed
inputs, lethal inflictor identities and corresponding effects support
a likely shared activation for this particular triple. The demo
does not explicitly link the activation to both projectile serials.
The death event reports item definition 1151, while the first effect
in the packet explicitly reports 207. These fields are retained as
separate evidence. [Valve secondary-fire bit](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/in_buttons.h#L12-L24),
[launcher detonation event](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_weapon_pipebomblauncher.cpp#L396-L431)

## Audit method and controls

Replay the optional sidecar in packet, message, and entity order.
Resolve the death's inflictor at its message and match the live entity
index and serial. A leave or deletion ends the observation interval.
Count each victim once per projectile. Check the credited attacker
against the full thrower handle and the unique player inventory
holding the original launcher. The same-tick lethal grouping used
the recorded packet, server tick, player lifetime and launcher handle.

Compare explicit zero-delay explosion origins against the pipebomb
positions present at the beginning of that packet. The offline effect
check used eight map units for this spatial comparison and required
one plausible complete-origin match in each direction. A partial
origin was diagnostic only. Keep ambiguous and repeated records out
of the effect groups. The existing compact explosion writer skips
packets with multiple explosion records, so the audit reads the raw
`TempEntities` records from `packets.ndjson`. Effect origins and
contact entity fields carry no projectile serial or activation ID.
[Grenade effect and damage attribution in Valve's SDK](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_weaponbase_grenadeproj.cpp#L293-L356)

The offline checks covered the real cases in the table and synthetic
perturbations of those packets. A repeated death preserved the unique
victim count. Distinct owners stayed separate even with a shared
numeric launcher value in the control. Conflicting launcher ownership,
an unknown serial, and a changed launcher blocked the affected death
join. The same actor's records in separate packets retained separate
groups even on the same server tick. Duplicate effects, spatial ties,
and missing origin axes stayed unresolved. The real likely air
detonation remained a two-victim single-projectile case.

## A06 proposal boundary

Start with the per-projectile planted-kill evidence for any future trap
tag proposal. Keep dwell and identity confidence per bomb, record
distinct casualties, and preserve unknowns through visibility or
ownership gaps. Validate proposed minimum dwell against reviewed cases
before changing scoring.

Same-packet, same-owner multi-projectile evidence can be reported
offline with its provenance. An actor marker, input bit, or spatial
effect narrows the context. A confirmed shared activation requires an
independent link from one activation to the relevant bombs. Current
replays support likely associations and distinct casualty counts.
Automatic multi-bomb scoring remains pending.

[Demo 1530633](https://demos.tf/1530633),
[pinned comp fixture](https://github.com/demostf/parser/blob/e1425a7cf72e1656fa8cceea14673f56b59875e2/test_data/comp.dem),
[player-published POV archive post](https://www.teamfortress.tv/49305/i63-pov-demos-thread/#18)
