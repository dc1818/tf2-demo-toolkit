# A03-P2 — complete Uber-save candidate pipeline

Status: approved by owner and implemented on `work/demo-toolkit-roadmap`.
See [native implementation and measured verification](uber-save-validation.md).
The original proposal alone did not complete A03; the linked runtime pipeline
and tests now replace the earlier diagnostic-only status.

## Requested result

A normal parse can produce an Uber-save candidate attributed to the Medic,
with the protected teammate, actual threat/protection/survival evidence,
and subsequent kills or objective results. Existing table, filters, Details,
Preview, recording controls and layout remain in use. Existing candidates
and scoring rules remain unchanged. Main must not be updated.

## Newly verified data path

Valve's TF player code records player_hurt.damageamount from actual health
lost. Invulnerable targets can therefore produce hit events with zero damage;
these prove a hit, not the otherwise-lethal damage amount. The per-condition
damage_prevented event is server-only and cannot be assumed present in demos.

However, DT_TFPlayerResource broadcasts a per-player m_iDamageBlocked array.
Valve's Event_PlayerBlockedDamage increments TFSTAT_DAMAGE_BLOCKED for the
provider, and the resource exposes the current-round total at approximately
one-second intervals. This is aggregate provider data, not a per-victim hit.
The array is present in both public fixtures' network tables. Actual counter
updates were found in comp's existing packets.ndjson. No weapon-damage guess
is needed to decode these totals.

The current parser/exporter does not expose that counter in state samples.
It already internally tracks medigun healing-target entity handles, but does
not expose them either. Both are needed to disambiguate shared/healed/flashed
patients. A target link alone is not proof of who provided invulnerability.

Example for review: comp deployment at demo tick 19796 identifies Medic user
78 (entity 7) and recipient user 62 (entity 6). The teammate is at very low
health, receives protected zero-damage hits, survives and later gets a kill.
Medic resource slot 7 subsequently changes from 140 to 183 total blocked
damage. Slot 17 belongs to a different player: never associate array slots
with user IDs, or attribute another Medic's counter to this deployment.
This example is not yet accepted as a verified save; timing/attribution must
pass the proposed tests first.

## Proposed additive exporter changes

1. Nullable damage_blocked_total: observed current-round counter, preserving
   unknown, counter resets and entity/user identity changes.
2. Optional observed medigun target evidence: distinguish unknown target from
   a decoded no-target state and a known target entity. Resolve entities to
   users only through same-epoch state mappings.

No old fields change meaning or disappear. Old exports remain supported and
are not silently reanalyzed. No existing condition or weapon logic is replaced.
Stats are delayed and delta-compressed: the last changed-property tick is not
necessarily the time of the blocked hit. Record this uncertainty explicitly.

## Detector and integration steps

1. Match deployment, provider identity and named invulnerability recipient,
   including subsequent flashes. Require actual protection-state evidence.
2. Reconstruct counter epochs and update intervals, allowing the documented
   stats-send delay. Exclude unknown prehistory and resets.
3. Attribute a counter increment only when the protected-hit evidence and
   provider/target history make its recipient unambiguous. Multiple possible
   recipients/providers, shield effects or incomplete coverage stay unknown.
4. Build an observed-health counterfactual accounting for damage, healing and
   health changes. Do not simply subtract a whole aggregated counter from the
   deployment's starting HP. Require evidence that credited blocked damage
   exhausted the no-protection health balance while the actual target lived.
5. Track recipient kills and supported team-objective/round followups within
   an initial eight-second outcome window, stopping at round/identity breaks.
   A later capture by itself is not evidence of a save or proof of causality.
6. Emit a separate zero-kill Medic candidate tagged uber_save only when the
   save is supported and a meaningful followup exists. Supply deployment,
   rescue and outcome ticks through the existing recording cue mechanism.
7. Use an initial score of 30 for this new candidate type, consistent with the
   new non-kill crossbow-save base. Do not change scores of existing candidates
   or add speculative outcome bonuses. Include all reasons/evidence in Details.
8. Add tests and compare existing candidates with the detector disabled/enabled.
   Back up every checkpoint on the work branch; keep draft PR #62 unmerged.

## Required verification before marking A03 implemented

- Single protected teammate receiving otherwise-lethal blocked damage.
- Multiple same/different-team Medics; misleading healer-list events.
- Multiple protected patients or self hits within one counter interval.
- Flashing, lingering invulnerability, provider death and target switches.
- Kritzkrieg, Quick-Fix, resistance bubbles, MvM shields and unrelated buffs.
- Counter reset, delayed update, user-ID/entity reuse and missing data.
- Intervening heals/overheal caps, actual death, and ordinary safe deployments.
- Recipient kill/objective outcome, unrelated late outcomes and round boundaries.
- POV/STV scope, tick namespaces, cancellation and performance/space costs.
- Normal candidate export, filtering, preview/recording cues and legacy parity.

If the available counters still cannot support unambiguous lethal-damage
attribution, do not claim completion or silently substitute a probable-save
heuristic. Present the measured limitation and a separate proposal instead.

## Approval scope (subsequently approved)

Approve the two additive evidence fields and the supported-evidence candidate
pipeline above. This does not authorize a UI redesign, a heuristic fallback,
changes to existing scores, or a merge/release from main.

## Primary sources reviewed

- https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_player.cpp
- https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_gamestats.cpp
- https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/server/tf/tf_player_resource.cpp
- https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/shared/tf/tf_player_shared.cpp

SDK behavior is a reference, not proof that every historical demo build emits
identical data. Verify against the actual decoded demo network tables.
