# A13 weapon attribution baseline and compatibility proposal v1

November 26, 2026. Phase 2 baseline `3c30439` on
`work/ralph-recovery-pilot`. Ralph reviewed the exporter, analyzer, item
schema fallback, presentation, and scoring paths. Approval for the registry
implementation is pending.

## Baseline method

The Windows exporter and analyzer were built from the pilot source using
Rust 1.88. A disposable entry point called the production analyzer and wrote
an auxiliary trace of the accepted deaths before state enrichment. Each
export was analyzed twice against the same input, with the schema argument
absent and with a copy of the isolated TF2 installation's
`tf/scripts/items/items_game.txt`. The real candidates include the normal
state enrichment and scoring. The synthetic single-kill trace uses default
state and provides 15 controlled comparisons of attribution rules. The
synthetic run with schema reproduced its trace byte for byte. All 17
existing analyzer tests passed on Windows, including the comparison of
The Original and stock rocket behavior with projectile evidence.

The real corpus includes a recent SourceTV replay and the same pinned STV
and player POV fixtures used for the prior protection and crossbow work.
[SourceTV demo 1530633](https://demos.tf/1530633),
[comp STV fixture](https://github.com/demostf/parser/blob/e1425a7cf72e1656fa8cceea14673f56b59875e2/test_data/comp.dem),
[gully POV fixture](https://github.com/demostf/parser/blob/e1425a7cf72e1656fa8cceea14673f56b59875e2/test_data/gully.dem)

## Native measurements

| Fixture | Accepted live-scope deaths | Candidates in each schema mode | Candidates with any changed field |
| --- | ---: | ---: | ---: |
| 1530633 SourceTV | 199 | 67 | 0 |
| comp SourceTV | 243 | 58 | 0 |
| gully player POV | 32 | 11 | 0 |

All 136 candidate records matched in order and content, including their
tags, scores, and score breakdowns. The POV player's scope contained no
sticky kills. The comp fixture contributed 16 accepted remote-sticky deaths
and one accepted death each with `deflect_promode` and `deflect_rocket`
as the raw weapon tokens.

The fresh no-schema rerun of demo 1530633 matched the earlier saved
67-candidate export field for field after normalizing the source path. Its
199 accepted deaths included 20 remote-sticky tokens and 34 occurrences of
The Original. Six remote-sticky kills and 21 kills with The Original
appeared in its candidate output. For example, candidate `r2-p4-t47584`
contains a single remote-sticky kill at demo tick 22525 with score 30 and
tags `late_round` and `round_clinch`. Candidate `r3-p4-t72309` contains
another at demo tick 47250 with score 28 and tag `medic_pick`.

The [machine-readable baseline](weapon-attribution-baseline.json) records
fixture and schema hashes, retained event and state stream hashes, and the
synthetic inputs and results. Candidate hashes use one compact,
ASCII-escaped JSON object per line, with keys sorted recursively and
`source_demo` replaced by the fixture key. List order is preserved.

## Controlled comparisons

These scores are for isolated synthetic one-kill groups before state
enrichment and candidate filtering. The full real-candidate scores come
from the native analysis pass.

| Control | Legacy weapon value | Weapon tags | Isolated score |
| --- | --- | --- | ---: |
| Literal stock stickybomb launcher | `stickybomb_launcher` | `projectile_kill` | 10 |
| Remote sticky, stock or reskin definition | `tf_projectile_pipe_remote` | None | 10 |
| Literal Quickiebomb Launcher | `quickiebomb_launcher` | `projectile_kill` | 10 |
| Grenade launcher | `grenadelauncher` | `pipe`, `projectile_kill` | 18 |
| The Original | `quake_rl` | `projectile_kill`, `rocket` | 18 |
| Stock and upgradeable rocket controls | `rocketlauncher` | `projectile_kill`, `rocket` | 18 |
| Unknown explicit token with a known item index | `fixture_unmapped` | None | 10 |
| Loose Cannon impact control | `loose_cannon_impact` | `projectile_kill` | 18 |

The full schema adds the Quickiebomb secondary slot. The raw name and its
score stay the same for this control. The conflicting remote-sticky case
also retains its definition index and custom-kill code.

### Schema fallback defect

When the synthetic event provides only `weapon_log_class_name`, its
normalized weapon is empty without schema. The legacy spelling
`weapon_logclassname` works when present without a higher-priority
`weapon` field. With schema supplied, the native-spelling control and a
control with an explicit empty `weapon` both fall back to
`cp_egypt_final`. The local schema contains item definition 20 for the
stock Stickybomb Launcher and another numeric key 20 in its map data.
The existing line-based parser walks numeric keys without section scoping,
allowing the later map block to overwrite the item entry. The baseline
records that behavior for a separately approved correction. The schema
trace also returned `cp_steel` for definition 19 and `cp_cargo` for
definition 207 alongside real remote-sticky events. Their explicit raw
weapon names took precedence, so these fallback values did not replace
the weapon text in those events.

## Current attribution boundaries

- Raw game-event weapon text, weapon ID, definition index, custom kill and
  projectile type are distinct evidence. A real current death event gives
  the raw weapon `tf_projectile_pipe_remote` and definition index 19.
  The installed schema lists index 19 under the stock Grenade Launcher.
  The raw projectile text and item metadata must both remain available to
  the caller.
- The current analyzer lowercases the first available legacy weapon field.
  An empty result can fall back to the item name for a definition index.
  An empty slot can fall back independently. The analyzer's fallback field
  is `weapon_logclassname`, while the native exporter writes
  `weapon_log_class_name`. The item parser reads fields in individual
  definitions and can miss inherited prefab data.
- The Original keeps the legacy export value `quake_rl` and its display
  name. The classifier aliases it to `rocketlauncher` where the existing
  compatibility helper is called. Raw `unique_weapons` metrics and the
  stock versus reskin distinction remain meaningful.
- The interface labels `tf_projectile_pipe_remote` as Stickybomb Launcher.
  The analyzer's exact-name projectile tag already covers
  `stickybomb_launcher` and `quickiebomb_launcher`. Projectile-entity
  matching and sequence bonuses use other predicates with different
  boundaries. Custom-kill values and secondary effects have separate rules.

## A13 compatibility proposal v1

Move the existing `quake_rl` classification alias and the three duplicated
exact-name weapon tag lists into one internal registry lookup. The per-kill
and group functions currently share identical lists for `projectile_kill`
(21 values), `pipe` (three), and `rocket` (five). For the first slice, preserve
each entry in those lists. Leave entity matching, airborne evidence, every
score predicate, item-schema fallback, custom-kill interpretation, and the
displayed label lookup as their current callers use them.

This creates one place to review the existing sticky tag rules. Collapsing
all names into one canonical string would conflate the raw event evidence
with classification and presentation. Changing the log-class fallback,
inherited schema behavior, or enabling a tag for remote stickies requires
a separate measured proposal.

### Acceptance

- Run the native STV and POV corpus with each schema configuration.
  Compare candidate order and identity, grouping and clip bounds, kill
  evidence, weapon and slot fields, per-tick and group tags, metrics,
  scores and score breakdowns to the corresponding baseline.
- Test stock rocket and `quake_rl` with and without matched projectile
  evidence. The Original must retain its original weapon value and label.
- Test literal sticky launcher names, remote-stickies, a reskin, a same-item
  conflict, unknown and empty text, class-name-only input, missing schema,
  and a secondary-effect control. Current classification outcomes stay
  reproducible in compatibility mode.
- Preserve the candidate and recording schemas. A source revert rolls back
  the refactor. The first slice requires no migration or reparse of stored
  exports.
