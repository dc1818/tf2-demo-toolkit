# Opt-in sticky evidence export

The Phase 2 A06 parser prototype is available through the native exporter.

```text
export_all <input.dem> <output_directory> --sticky-evidence
```

The export manifest lists `files.sticky_evidence` and
`sticky_evidence_format_version` when requested. The sidecar is
`sticky_evidence.ndjson`, version 1. Readers should use the manifest to
discover the file when a directory is reused. Use a fresh output directory
to compare independent exports.

## Reading the sidecar

- Rows carry `packet_sequence`, `demo_tick`, the nullable packet
  `server_tick`, and the decoded `message_index`. Entity rows also
  preserve order within the entity message. Explicit removals follow the
  entity updates in that message. Use the ordering keys when several
  observations share a tick.
- Entity rows distinguish `enter`, `delta`, `leave`, `delete`,
  `removed`, and `replaced`. Pipebomb rows cover the decoded pipebomb
  class across all modes. An entry carries the network serial, including
  zero when reported. Later rows retain the known serial from that entry.
  Rows without a known entry carry a null serial. A leave ends the
  visibility interval. A new entry at the same index starts a fresh
  observation interval even if the serial matches. Round-start events
  clear the tracked pipebomb identities.
- Pipebomb observations carry raw pipe mode, touched state, position,
  team, defensive status, deflection count, and available full owner,
  thrower, and launcher handles. Each field records whether its value
  was explicit in the message or supplied by the parser's entry
  baseline. An omitted observation supplies no value for that field
  in the row. Keep explicit zero distinct from absence. Apply deltas in
  order within an observation interval.
- Player observations carry the entity index and serial, the nullable
  user ID known at the observation, and available primary, secondary,
  and melee handles. For ownership checks, use the full handles and the
  time-valid player association. An unresolved player or conflicting
  owner observations leave the attribution uncertain.
- Game-event rows link to the original `events.ndjson` by packet
  sequence and `event_index_in_packet`. They mark round transitions,
  deaths, detonation signals, sticky destruction, and deflections.
  The first message index includes temporary-entity messages. The
  legacy event index uses the existing ordering without them.
  [Source capability matrix](sticky-trap-source-capability.md)

The sidecar records observations for future A06 analysis. A visibility
exit or entity removal leaves the end cause unknown. A touched interval
can establish a lower bound on planted dwell when the same visible
lifetime, position and ownership evidence persist. A death with a known
inflictor index and serial can be tied to that observed projectile.

## Windows fixture validation

All 239 Rust workspace tests passed, including cases for entry-baseline
provenance, explicit overrides, zero and unknown serials, leave/re-entry,
removal and deletion, index reuse, and event ordering across a round
boundary. On the three retained A13 demos, the optional export matched
all eight legacy output files byte for byte.
The default and opt-in runs for demo 1530633 also matched each other,
and the default manifest listed the original files. A native analyzer
replay of the opt-in `gully` export produced the same 11 candidate
records byte for byte.

| Fixture | Recorded remote-sticky deaths | Visible damaging-sticky serial joins | Matches full launcher handle | Matches full thrower handle | Joins with planted dwell bound |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1530633 SourceTV | 20 | 20 | 20 | 20 | 17 |
| comp SourceTV | 16 | 16 | 16 | 16 | 12 |
| gully POV | 18 | 7 | 7 | 6 | 6 |
| habib i63 grand finals POV | 35 | 21 | 21 | 21 | 14 |

The fixture audit matched each sidecar event to the legacy packet,
event index, and timestamp. It joined deaths against the visible
inflictor at the event's message, with known serial and raw pipe mode 1.
For the dwell bound it tracked touched samples in the visible interval
and reset on changed position, touched reset, or changed ownership
evidence. Launcher matches compared full handles to uniquely matching
sampled player inventories. Thrower matches also checked the serial
encoded in the player's network handle. [Valve network handle format](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/public/const.h#L57-L79)

### Labeled Demoman POV cases

The player-published i63 archive contains
`habib_i63_grandfinals_2018-08-26_22-31-53.dem` on
`cp_process_final`. Its SHA-256 is
`573861cf89c10b36a6f9d525a84dbcc48d1d22d89ae7e58e0db2f9d7057daf3c`.
The server-info player slot resolves to entity 12 and user ID 80 in the
sidecar. Of the recorder's 27 remote-sticky deaths, 17 had a visible
damaging-sticky serial join. Eight were live-round deaths with a
positive planted bound and agreeing launcher and thrower evidence.
[Player's fixture post](https://www.teamfortress.tv/49305/i63-pov-demos-thread/#18),
[original demo archive](https://www.mediafire.com/file/rmwoxn1qrqimga1/habibi63grandfinals.zip/file)

| Case | Exact observation |
| --- | --- |
| Planted sticky with two victims | At demo tick 105576, packet 245151, message indices 1 and 3 credit user 80 for deaths of users 70 and 74. Both name inflictor 633, which was visible in mode 1 with serial 83. Both full-handle checks agree. The last observed untouched tick was 155998, the first touched tick was 156071, and the deaths were at server tick 156666. The position and observed ownership persisted for a planted lower bound of 595 ticks (8.925 s). |
| Two victims with unknown planted dwell | At demo tick 154193, packet 350532, message indices 1 and 3 credit user 80 for deaths of users 70 and 73. Both name inflictor 1190, serial 366, with matching ownership. The observed interval leaves planted dwell unknown. |

These labels describe one projectile and two casualties in each case.
Separate bombs in the same activation still require labeled examples and
an attribution rule. The source measurements support planted evidence
as input to a future detector proposal.
