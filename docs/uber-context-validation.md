# A03 — deployment context and feasibility checkpoint

## Scope

Read-only offline probe; no app integration, tag, score, UI, default or export
contract changes. Run with Python 3.10+:

```sh
python3 tools/probe_uber_context.py EXPORT --tick-rate 66.6666666667
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests -v
```

The rate shown is verified from the public fixtures' server-info interval.
Pre-health requires an alive, in-PVS snapshot from a prior packet within
500 ms. The probe reports damage in the preceding second and observations
during up to eight seconds after deployment. Windows end early on a new
deployment by the same medic, round boundary, clock regression or end of
export. A recipient/medic life or identity boundary stops attribution within
that context. Event absence does not prove survival.

Only the deployment's initial target is tracked. Later flashes onto other
patients are not reconstructed. Same-team kills require recent observed team
membership; recipient kill rows are raw event context, not tactical-impact
scores. An invulnerability event identifies a named medic and recipient but
does not establish uninterrupted protection, a blocked hit or a prevented
death. All diagnostic rows explicitly report life_saved=unknown and
damage_prevented=null.

## Findings

The current PlayerStateSample contains health, movement, and medigun
information but no invulnerability/resistance condition fields. The
player_chargedeployed event identifies medic and initial target.
player_invulned supplies recipient and medic user IDs.

Valve's [TF player shared implementation](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_player_shared.cpp)
creates damage_prevented when a condition is removed. Its preventor/victim
values are entity indices (not user IDs) and FireEvent uses dontBroadcast=true.
Do not treat that event as a guaranteed demo record or correlate its IDs as
user IDs. The SDK snapshot is evidence of SDK behavior, not proof that all
historical game builds behave identically. Even optional conditions would
not reconstruct exact blocked damage or prove a counterfactual death.

## Tests

13 synthetic tests passed. They exercise context measurements without save
claims, same-packet invulnerability ordering, wrong medic, Kritzkrieg,
unknown survival, death/respawn, round boundaries, stale/same-packet health,
entity reuse, clock regression, repeated deployments/time limit, invalid
rates, and malformed export input. The tests also compare input file bytes
before/after to verify that probing does not modify exports.

The audited exporter binary was recovered from the Linux package for
commit 568e2434c4aa04367cc6b767f2f0cabf3f7e62aa, workflow 40052337653.
Public demo fixtures were recloned from demostf/parser. POV/STV classifications
come from the generated manifests. Both demos were freshly exported and
processed successfully.

| Fixture | Deployment rows | Fresh target pre-health | Named recipient invuln event within window | Recipient kill window |
| --- | ---: | ---: | ---: | ---: |
| gully (POV) | 19 | 14 | 17 | 8 |
| comp (STV) | 22 | 17 | 19 | 6 |

Neither event export contained damage_prevented.
In comp at demo tick 19796, the initial target's pre-deployment health was
2, a matching invulnerability event followed, and the target subsequently
had one kill within the observation window. This is useful review context,
**not a verified life saved**. No playback labels or precision/recall claims.

## Proposal A03-P1 — approved November 5, 2026

Owner response: “yes do that.” Approval covers this narrow exporter extension,
not new candidate generation, scoring or UI. Approved scope:

- Add optional player snapshot fields invulnerable,
  invulnerability_wearing_off, uber_bullet_resist, uber_blast_resist,
  uber_fire_resist, reading the existing PlayerCondition machinery.
- Keep missing/unknown condition evidence distinct from false. Establish
  condition-validity provenance first; where decoder validity cannot be
  established reliably emit null, not a manufactured false.
- Preserve every existing field and behavior. Old exports remain readable.
  The app's analyzer and UI ignore these additions until separately approved.
- Extend offline diagnostics to check protection near the threat and any
  follow-up survival/outcome observations. Do not add save tags, scores,
  candidate generation, provider/beam reconstruction, or new UI controls.
- Test old/new export compatibility, actual condition transitions, PVS loss,
  respawns, stock/Kritz/resistance types, and before/after field parity.
  Measure added bytes and export runtime before accepting overhead.
- Reversal: stop emitting the optional fields; existing fields and exports
  require no migration or deletion.

Alternative: retain context-only diagnostics with the current exporter.
Neither path can guarantee exact lethal-damage prevention from these demos.
Reliable prevention remains a separate evidence/feasibility checkpoint.

## Workspace recovery

Automated maintenance removed the prior local worktree and unpushed commits
c4033ba, ece8d98, e314eca and 174a725. Their earlier results remain in chat,
but those original commit objects and files were not recovered from GitHub.
Do not report the old 21-test suite as rerun in this checkout. This worktree
was restored from the published 568e243 baseline; the saved original guideline
is included unchanged. A01/A02 prototypes must be reconstructed and reverified
before roadmap integration. This A03 module is standalone.

Owner authorization remains: work on a separate branch; never update main
until all work is complete and the owner explicitly approves.
