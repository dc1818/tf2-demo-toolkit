# A06 planted sticky kill tag trial

Ralph reviewed this offline proposal against pilot revision `e4d1fce`.
The trial evaluates `planted_sticky_kill` against existing clips. It
prints a JSON comparison with separate baseline and trial tags and
scores for each affected clip.

```text
python tools/prototype_sticky_trap_tag.py <export> [--candidates <frag_candidates.ndjson>]
```

Use a version 1 sticky-evidence export and its candidate stream.
The trial joins a remote-sticky death at its ordered event message to
a visible projectile with a known serial and observed raw mode 1.
The full thrower handle and unique original launcher match must agree
with the credited attacker, and the current launcher must match the
original. A positive dwell bound starts at an observed touched sample
with position and handle evidence. Movement, ownership changes,
deflection, touched reset, visibility loss, and re-entry reset the
interval as applicable. A death below the threshold has insufficient
observed dwell for the tag. Its true dwell can be longer.

Candidate matching uses packet sequence, event index, attacker, victim,
and demo tick within one export. The candidate stream supplies the
existing grouping and analysis scope. The fixture counts below include
recorded remote-sticky deaths across all visible players. Native Habib
analysis used its all-player fallback because the header nickname could
not be matched to the retained roster. The independently confirmed
recorder, user 80, owns the trial clip in that demo.

## Threshold sweep

Cells show qualifying recorded deaths and existing clips receiving the
trial tag. The four exports contain 207 baseline clips in total.

| Minimum observed dwell | 1530633 STV | comp STV | gully POV | Habib POV |
| --- | ---: | ---: | ---: | ---: |
| 0.5 s | 6 / 2 | 6 / 1 | 4 / 0 | 4 / 1 |
| 1 s | 5 / 2 | 4 / 1 | 2 / 0 | 3 / 1 |
| 2 s | 4 / 2 | 4 / 1 | 2 / 0 | 2 / 1 |
| 3 s | 4 / 2 | 3 / 1 | 1 / 0 | 2 / 1 |
| 5 s | 3 / 2 | 3 / 1 | 1 / 0 | 2 / 1 |

All five thresholds flag the same four clips. At 2 s, 12 recorded
deaths meet the strict per-projectile rule and five of those deaths
appear in the four candidate clips. Of 89 recorded remote-sticky
deaths, 49 had usable dwell evidence. The remaining 40 include 16
unknown identities, nine unknown modes, 14 unknown dwell intervals
and one unresolved ownership match.

## Before and after at 2 seconds

| Fixture and clip | Baseline tags | Trial addition | Score before → trial |
| --- | --- | --- | ---: |
| 1530633 `r3-p4-t72309` | `medic_pick` | `planted_sticky_kill` | 28 → 28 |
| 1530633 `r5-p5-t134325` | `medic_pick` | `planted_sticky_kill` | 28 → 28 |
| comp `r1-p76-t65222` | `medic_pick`, `uber_drop` | `planted_sticky_kill` | 48 → 48 |
| Habib `r6-p80-t156666` | `demoman_pick`, `multi_kill`, `rapid_sequence`, `three_kill` | `planted_sticky_kill` | 83 → 83 |

- The planted triple adds one trial tag to its clip. Its two
  confirmed kills from serial 83 share the 8.925 s lower bound.
  The third casualty has an unobserved pipe mode and remains
  outside the strict tag evidence.
- The probable air-detonation double preserves both deaths with
  unknown planted dwell. It yields zero qualifying kills.
- SourceTV packet 69757 contains six planted bombs attributed to
  user 62 and zero sticky deaths. The trial emits no kill tag from
  those effects.

These controls reuse the exact labeled packets in the
[multi-bomb audit](sticky-multi-bomb-validation.md). The prototype
also verified score equality and that each affected clip gains only
the proposed tag across all five thresholds.

## Proposed first integration

Use **2 seconds of observed planted dwell** as a provisional starting
point for a tag-only proposal. It excludes the 0.66 s and 1.365 s
Habib observations while retaining the reviewed planted double and
the same four trial clips. The clip list was the same at each threshold,
so the corpus leaves the optimum dwell threshold open.
Use a labeled held-trap and short-placement corpus to calibrate the
threshold before assigning points. Multiple eligible casualties add
the tag once to a clip. Keep shared activation and per-projectile
qualification as distinct evidence.

## Approved native tag

The owner approved the 2-second tag-only integration. The Toolkit
requests the version 1 evidence sidecar during normal demo parsing.
The analyzer attaches `planted_sticky_kill` to the existing candidate
and qualifying kill tick after candidate selection and ranking. The
tag participates in filtering and kill-tick details. Recording folders
continue to follow the previous primary-category rules. Existing
exports without a supported sidecar remain valid analysis inputs.

All 240 Windows Rust workspace tests passed. The native regression
exercises 133 versus 134 ticks at 66.67 ticks per second, movement
interruptions, unknown pipe mode, an ownership mismatch, legacy
exports, and malformed optional input. It checks score, clip bounds,
and primary-category preservation alongside candidate and kill-tick tags.

Native analyzer replays on all four Windows fixtures retained all 207
candidate records in the same order. The only candidate differences
were the expected tag on four clips and its matching kill-tick groups.
All clip bounds, scores, score breakdowns, and primary categories
matched the baseline.
