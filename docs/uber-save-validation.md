# Life-saving Uber candidates — A03

Implemented on `work/demo-toolkit-roadmap` under approved proposal A03-P2.
Owner approval: “yes go ahead remember how the program works you parse all of
the demos information and use it to show candidates for frag clips which could
include the medic things i mentioned above on top of all of the other features.”
This completes the supported-evidence native candidate pipeline; it is no
longer only the earlier offline deployment-context diagnostic. Owner playback
review and live TF2/HLAE recording verification are still outstanding.

## Normal workflow and preservation

Normal parsing exports the two approved observations. Normal analysis appends
independent Medic candidates tagged `uber_save`, alongside existing frag,
bookmark and `crossbow_save` candidates. No existing candidates are edited,
merged into a save, rescored or removed. Existing table, comma-separated tag
search, filters, Details, single-candidate preview, recording and import paths
remain in use. No UI source or layout is changed.

A new candidate has score 30, zero kills, the Medic as actor and the saved
player in its metrics/evidence. Follow-up tags distinguish `recipient_kill`,
`team_objective_followup` and `team_round_win_followup`. An objective follow-up
means an observed same-team event, not proof that the rescue caused the win.
Deployment, first/last threat, counter confirmation and outcome demo ticks are
recording cues, so zero trailing padding does not omit the follow-up. Evidence
in Details contains all contributing hit events, actual HP at each hit,
provider identities, snapshots, counter values and the chosen lethality bound.

Previously exported candidate files remain loadable without automatic
reanalysis. Older raw exports lacking the observations do not produce guessed
Uber saves. A fresh parse is needed to obtain evidence absent from an old
export. All the other candidate detectors continue to operate.

## Additive export contract

| Field | Values | Meaning |
| --- | --- | --- |
| `damage_blocked_total` | unsigned integer or null | Observed current-round `m_iDamageBlocked` resource counter, matched by **entity slot**, never by user ID. Null is not a zero baseline. |
| `medigun_target` | null | No usable target observation, including unresolved/malformed/world handles or unavailable Medic state. |
| `medigun_target` | `{"entity_id":null}` | Explicit decoded no-target EHANDLE sentinel. |
| `medigun_target` | `{"entity_id":6}` | Observed target entity; resolve to a user through the current identity epoch. |

Both caches are separate from the legacy fields and excluded from serialized
`Player` data. Target provenance resets on entry/reentry, PVS loss, life/class
change, gun entry, removal and user/Steam replacement. Counter observations
are global scoreboard data and need not be in PVS; removal/identity replacement
clears the cache. Malformed/negative counters are unknown. Counter decreases
break the analysis epoch. Existing `has_condition`, legacy target lookup,
Medigun type and weapon behavior are preserved.

## Evidence rules

1. Require a fresh alive/in-PVS Medic, observed stock invulnerability Medigun,
   deployment in a live round, and the established POV-player scope. Non-stock
   charges and MvM maps are excluded. Unknown clock/interval, missing state or
   parser-reported incomplete demos produce no new save candidate.
2. A protected hit needs a named `player_invulned` provider/recipient link,
   actual invulnerability/wear-off observation, and a known beam-to-entity
   history (or the Medic's own protection). Explicit detach/target switches
   allow at most one second of lingering link history; unknown targets clear
   that history. A second possible provider makes attribution ambiguous.
3. A counter increase has an attribution window of at most 1.5 seconds. This
   accommodates the SDK's roughly one-second resource send period. Repeated
   delta-compressed values are **not** treated as newly acknowledged stats.
   Keep an unsettled-hit marker until a genuine counter change; if an older
   possible contributor falls outside the detail window, reject the interval
   instead of assigning its damage to a recent hit. Missing/old baselines,
   counter resets, identities, deaths and round boundaries cannot be bridged.
4. All possible contributing protected hits on the provider's team must name
   one recipient and exactly one provider. Unknown recipients/teams, another
   same-team protected patient, self hits mixed with another patient, or
   positive damage with unknown/protected provenance reject attribution.
   Known opposing-team hits do not belong to this provider's counter.
5. Prove lethality using either of two conservative bounds. Do not divide an
   aggregate counter into invented per-hit damage or estimate weapon damage.
   **Individual bound:** blocked delta at least the sum of the patient's HP
   at every potentially contributing zero-damage hit proves at least one
   hit would exhaust HP (pigeonhole bound). **Cumulative bound:** otherwise
   require a fresh, continuous, nondecreasing HP curve strictly below a
   constant base maximum, no actual intervening damage, and blocked delta at
   least the final actual HP. The observed HP gain includes intervening
   healing; capped healing, overheal, decreases and gaps reject this bound.
6. Require the same living recipient at confirmation and at an observed
   recipient kill or same-team objective/round result. Outcomes must occur
   after the threat and within eight seconds of its first hit, bounded by the
   round end. Retain eligible outcomes that precede a delayed counter update.
   Confirmed rescues can still gain a follow-up after the Medic dies; patient
   death/identity breaks stop further attribution. Death after an already
   qualifying follow-up does not erase that completed rescue.

Freshness/continuity tolerance is 250 ms. Time uses the exported server tick
interval; recording cues use demo ticks. Incomplete/ambiguous cases remain
unknown; there is no probable-save fallback. These are bounded counterfactuals
using the observed health/actions/damage, not claims about every alternative
way players might have behaved without protection.

## Automated verification

Rust 1.88, actual source through a temporary headless analyzer harness:
**77 tests passed**, including **26 A03 detector/integration tests** and the
existing crossbow, frag, bookmark, model, export-preference and scheduler tests.
The harness uses the real modules with an unused temporary config-path shim;
it is not a full native GUI or recording test.

Parser: **14 library + 2 exporter tests passed**. These cover entity-slot
counter decoding, unknown versus zero, malformed/negative counters, target
resolution/detach/unresolved/world/malformed handles, visibility, life reset,
serialization and legacy readers.

Python: **18 tests passed**, including four additive-state parity validator
cases. Default comparisons still reject unapproved extra fields; the explicit
`--uber-evidence` option validates and excludes the two newly approved fields.

A03 coverage includes single lethal hits; cumulative two-pipe threats;
intervening healing, caps and decreases; routine safe Ubers; absent baseline;
resets and delayed acknowledgements; multiple patients/Medics; misleading
entity/user IDs; unknown recipients/targets/protection; opposing-team hits;
flashes and lingering target history; non-stock charges; Medic/patient death,
PVS/life/class/team/identity changes; entity reuse; round/time bounds;
kill/objective/round follow-ups; healer POV scope; duplicate rescue suppression;
missing clocks, incomplete demos, MvM exclusion, cancellation; normal candidate
serialization/reload, demo-tick cues and exact preservation of legacy candidates.

## Real fixture results

Public corpus: `demostf/parser` commit
`e1425a7cf72e1656fa8cceea14673f56b59875e2`, `test_data/comp.dem` and `gully.dem`.
No user/private demos or large exports are committed.

| Fixture | Capture | Prior candidates preserved | New Uber saves | Existing crossbow saves |
| --- | --- | ---: | ---: | ---: |
| comp | STV | 57 | 1 | 1 |
| gully | POV | 11 | 0 | 0 |

With A03 disabled/enabled on the same newly generated exports, every existing
candidate matched **exactly**, including score, grouping, tags, existing
evidence and ordering. Comparing to pre-A03 exports also matched after
excluding only the two additive evidence keys and normalizing the equivalent
source-demo pathname. Reconstructed legacy player/projectile state matched
after every packet in the union of both streams: 85,825 STV and 102,581 POV
packets. Additional optional-only state deltas do not erase old state.

Real positive: Medic user 78 / entity 7, patient user 62 / entity 6. Deployment
is demo 19796 / server 46932. Six protected hits through demo 19828 have HP
3, 5, 6, 7, 8 and 8 (sum 37). The Medic's counter changes 140 → 183 at demo
19832 / server 46968: delta 43 exceeds 37, proving an individually lethal
hit without inventing its exact damage allocation. Patient HP at confirmation
is 9. That same player kills enemy user 72 at demo 20298 / server 47435.
The candidate contains all these cues and its supporting events/snapshots.
The other Medic's entity slot 17 is never confused with user 78.

| Measurement | STV comp | POV gully |
| --- | ---: | ---: |
| Previous state-stream bytes | 705,915,638 | 409,920,939 |
| New state-stream bytes | 761,269,492 | 441,090,892 |
| State-stream growth | 7.84% | 7.60% |
| Baseline analyzer wall time | 29.087 s | 11.756 s |
| Analyzer with A03 wall time | 46.587 s | 17.736 s |

Timings are single sequential observations in this environment, not a controlled
cross-machine benchmark or a guaranteed ETA. The new detector makes an
additional streaming pass and keeps bounded recent state rather than loading
the whole export. Existing disk preflight, dynamic worker scheduling and timing
history remain unchanged; successful run measurements continue feeding the
normal estimate calibration. First-run estimates are not newly calibrated by
these two fixtures. Growth percentages above apply to the state stream, not
the entire export.

## Remaining validation and scope limits

No independent visual ground-truth labels, native Windows TF2 playback, or
HLAE media/config restoration tests were run in this environment. Existing
preview/recording cue integration is tested by candidate data and unchanged
consumer code, not by claiming an actual captured video. The POV fixture
produced no positive; positive healer-POV scope is covered synthetically.
Counter/target absence, ambiguous multiple recipients/providers, capped healing,
long stats delay, sparse samples and historical demo differences reduce
coverage instead of manufacturing certainty. Other Medic rescue types remain
separate roadmap items. Main remains untouched and no beta is released by this
implementation checkpoint.

## Primary implementation references

- [Valve player damage handling](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_player.cpp): actual health lost in `player_hurt`; invulnerability can yield zero-damage events; `Event_PlayerBlockedDamage` receives `realDamage` after `ApplyOnDamageAliveModifyRules`, not a guessed stock-weapon number.
- [Valve stats](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_gamestats.cpp): provider blocked-damage accumulation.
- [Valve resource](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_player_resource.cpp): current-round counter array and periodic resource updates.
- [Valve shared conditions](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_player_shared.cpp): named protection handling; server-only `damage_prevented` is not assumed available in demos.

SDK behavior is a reference; actual decoded fixture fields and observed events
were checked separately. No weapon-damage lookup substitutes for missing data.
