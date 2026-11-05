# Demo Toolkit — Technical Implementation Guideline

**Version:** 1.0 • **Date:** November 4, 2026 • **Owner / product approver:** Dalton Cornwell
**Status:** Proposed guideline for review. No implementation authorized by creation of this document.
**Repository:** https://github.com/dc1818/tf2-demo-toolkit
**Audited baseline:** `568e2434c4aa04367cc6b767f2f0cabf3f7e62aa`

## 1. Purpose and authority

This document defines the work needed to complete the missing and partial features in the preceding source audit. It also tracks unresolved reported defects separately from missing features. It is intended to guide future coding sessions, reviews, regression tests and beta releases without losing existing functionality.

**The current task is documentation only.** Do not change application code, settings, data, UI, repository branches, releases or the user's TF2 installation as part of this document's delivery. Future implementation requires a subsequent instruction to begin. Approval of this document is not blanket approval of all proposed behavior changes or mockups.

### Mandatory preservation rules

1. Do not remove, disable, replace or silently simplify an existing feature. This includes functions that seem redundant or inconvenient to the new design.
2. Do not change existing behavior, defaults, scoring, candidate grouping, output naming, storage paths, shortcuts or data contracts without describing the exact before/after effect and obtaining Dalton's approval first. A listed request authorizes planning its change, not unreviewed collateral changes.
3. **Do not change UI layout or presentation without explicit approval of mockups.** This includes new columns, controls, text, labels, tooltip appearance, page placement, spacing, fonts, colors, sizing, breakpoints and window behavior. The UI requests below remain approval-gated.
4. Preserve the original TF2 visual identity and bundled typography. Preserve stationary main navigation and Recording Settings sub-tabs. Do not replace the Candidates layout or introduce app-wide scrolling.
5. Keep all items previously marked `//` completed. Do not reopen or reverse them unless a demonstrated regression requires a specifically approved correction.
6. Preserve legacy exports, recordings, user preferences, config backups and camera paths. Never require users to reparse or rerecord merely to retain previously available functionality.
7. If the plan cannot be fulfilled without changing existing behavior, stop the dependent implementation and ask. Continue independent read-only investigation and unaffected authorized tasks.
8. Do not treat silence, elapsed time, prior general publishing permission or approval of an unrelated feature as approval of a new behavior/UI change.
9. If new source differs from this baseline, audit the difference first. Do not overwrite newer work or restore the old baseline wholesale.

### Approval protocol

Before coding a gated change, produce a short change proposal containing:

- Requirement IDs and current behavior, with source/screenshot evidence.
- Proposed behavior and why the existing behavior must change.
- Features affected, alternatives, and the smallest compatible option.
- Migration, rollback and user-data implications.
- Tests and measurable acceptance criteria.
- For presentation changes: actual current screenshot plus proposed mockups at relevant compact and desktop sizes; all changed states, popups and overflow cases.
- A precise approval question identifying the proposal version. Record the answer and scope in the approval register.

Mockups must match the actual app and TF2 assets. Design images are not evidence of implemented behavior. Changes outside the approved scope require another approval. Internal implementation choices that preserve all observable behavior may proceed once implementation itself is authorized.

## 2. Document structure and research basis

This is a combined requirements specification, architecture/change plan and verification guide—not just a feature checklist. It uses the arc42 organization of goals, constraints, component responsibilities, runtime behavior, decisions, quality scenarios and risks. Significant choices should have Architecture Decision Records (ADRs). Google engineering guidance informs small, focused changes and review of design, functionality and meaningful tests. These sources describe engineering process; they do not establish that a TF2 detector or HLAE command is correct.

Research sources, accessed November 4, 2026:

- [arc42 documentation structure](https://docs.arc42.org/): architecture, constraints, runtime views, quality requirements and risks.
- [arc42 architecture decisions](https://docs.arc42.org/section-9/): record significant decisions and their rationale.
- [Google: Small CLs](https://github.com/google/eng-practices/blob/master/review/developer/small-cls.md): keep changes focused and independently reviewable.
- [Google: What to look for in a code review](https://github.com/google/eng-practices/blob/master/review/reviewer/looking-for.md): review design, functionality, complexity and tests.

Thresholds and implementation strategies below are **proposals**, not measured results or approved product decisions. No new detector is claimed feasible with certainty until the demo-data investigation passes.

## 3. Existing system and protected baseline

| Component | Existing responsibility / implementation entry point |
| --- | --- |
| `parser/src/` and `parser/src/bin/export_all.rs` | Demo decoding and event/state export. Verify actual emitted fields before designing detectors. |
| `app/src/analyzer.rs` | Candidate construction, state evidence, scoring, per-tick tags, bookmarks and match classification. |
| `app/src/models.rs` | Candidate and application settings models; compatibility defaults. |
| `app/src/batch.rs`, `scheduler.rs`, `preflight.rs` | Parse/analyze scheduling, resource profiles, estimates and disk checks. |
| `app/src/main.rs` | UI orchestration, filters, selection, load/export handling and recording status. |
| `app/src/recording.rs` | HLAE launch, recording identity, media finalization, config isolation/restoration and recovery. |
| `app/src/parse_exports.rs`, `paths.rs` | Parse export organization and path handling. |
| `app/ui/app.slint`, `options.slint`, `tf2-theme.slint` | App layout, controls, responsive states and theme. |
| `director/src/ui.rs`, `lib.rs`, `director/ui/director.slint` | MIRV command execution, keyframe synchronization, timeline and panel. |
| App/Director guide modules and Slint overlays | Onboarding steps, live anchors and progress. New UI must not break these anchors. |
| `.github/workflows/` and `packaging/` | Cross-platform checks, packages and release smoke tests. |

Preserve: Parse's three-step workflow; optional schema detection; Low/Normal/High behavior; time/space estimates; disk preflight; multiple demos/folder import; previous-export loading; comma-separated tag filtering; class/map/server/minimum-score filters; selection; recording padding; single-selection Preview and Details; automatic and manual recording; output formats/FPS; configuration recovery; campath load/save; onboarding and options; platform packaging.

The audited source already supports more than two kills per sequence, filtered Select All, manual HUD-off initialization, TF2 Quit, campath XML import, JSON bookmarks, voice muting, recording indexing and restoration machinery. Extend or validate those systems; do not replace them with weaker implementations.

## 4. Work tracking and definitions

Statuses: **Missing**, **Partial**, **Investigation**, **Proposal ready**, **Approved**, **Implemented**, **Verified**, **Released**. Existing code alone does not justify Verified. A passing unit test does not prove correct TF2 playback or capture.

Gate codes used below:

- **B:** behavior change approval before implementation, including score/ranking changes and new default candidate inclusion.
- **U:** UI mockup approval before presentation implementation.
- **D:** data/output migration proposal and approval where existing persisted behavior changes.

The entire backlog is Planned; no gate is approved by this document. For each ID track owner, baseline SHA, proposal/ADR, approval reference, dependency IDs, branch/PR, fixtures, test results, residual limitations and release version.

## 5. Shared technical foundations

### 5.1 Event evidence, time and identity

Before implementing new detectors, inventory each demo type's available fields: player/projectile positions, velocities, grounded state, health/max health, conditions, damage/healing events, medigun charge, weapon identity, team/round/respawn state and death attribution. Record missing and sampled fields rather than treating them as false/zero.

Keep demo ticks and server/event ticks explicitly distinct. Use the existing verified conversion path; establish tick rate/time mapping from available metadata and test any fallback. Do not correlate actors by display name. Handle reconnects, entity reuse, deaths, respawns and round transitions. Prevent evidence from crossing unrelated lives or rounds.

Proposed additive detector result: stable detector ID/version, event interval, actors/targets, related kill ticks, evidence references, measured quantities, confidence class, uncertainty reason and proposed score contributions. Keep per-event tags distinct from whole-sequence tags. Introduce new modules only where they improve isolation; no unrelated mass refactor.

### 5.2 Scoring and classification

Preserve baseline scoring as a named/versioned implementation. Develop new weights in offline comparison mode before proposing activation. Separate event detection, confidence, contextual impact and scoring so uncertain evidence cannot masquerade as confirmed causality. Publish score reasons per tick and avoid counting the same event repeatedly through overlapping detectors.

All thresholds—speed, distance, timing, damage windows, trap age, survival duration and bonuses—must be documented, calibrated on labeled demos, and approved when they affect output. Evaluate precision/recall by detector and context, not just an aggregate score. Rare phenomena without enough examples remain explicitly experimental; do not fabricate certainty.

### 5.3 Compatibility and migration

Use additive optional fields with explicit schema/analyzer versions and backward-compatible defaults. Preserve original export files; put reanalysis into a new versioned result or approved copy. Never silently rescore old exports on load. Older exports lacking evidence must still display, filter, preview and record.

Separate recording identity from score, rank, tags and filename. Any identity change must map legacy recordings to the new system without false matches. Store moves/renames in a reversible journal; never rewrite users' existing media trees automatically. Path changes apply to new outputs unless an explicit migration is approved.

### 5.4 Runtime, resource and command safety

Reuse adaptive worker limits and progress reporting. Prefer bounded temporal windows/indexes over repeated whole-demo scans. Honor cancellation and disk checks. Compare CPU, memory, elapsed time and export size against the baseline on identical data and hardware; report measurements before accepting regressions or choosing performance budgets.

Treat filenames, bookmarks and campath XML as untrusted input. Escape commands correctly, reject unsafe path/command interpolation, bound input sizes, and avoid external XML entities. Do not run arbitrary text from `events.txt`. Verify HLAE commands against the version actually packaged/used before implementation; engine semantics are a research dependency.

## 6. Detection and analysis requirements

All entries below inherit evidence, versioning, compatibility and scoring rules. **Gate B** applies before activating changed candidate generation/scoring; **U** also applies to any visible label/control changes.

| ID / status | Required implementation | Acceptance evidence and counterexamples |
| --- | --- | --- |
| A01 High-velocity kills — Missing | Measure attacker speed around the kill, duration and airborne context. Separate speed from inferred rocket-jump/surf cause. | Fast movement positive; ordinary sprint, teleport, respawn and isolated corrupt velocity negatives. Missing movement evidence stays unknown. |
| A02 Ubersaw charge gain — Missing | Correlate confirmed Ubersaw contacts with charge changes and patient/weapon state. Count nonlethal hits, not only kills. | Hit causing charge increase; miss, capped charge, deployment/drain and ordinary healing charge gain distinguished. |
| A03 Life-saving Über and outcome — Missing | Correlate activation/recipient protection with incoming threat, survival and later team impact. Distinguish rescue evidence from post-Über kills. | Protected likely-lethal scenario and two-pipe example; routine Über and unrelated later win negatives. Report likely/prevented observed damage only where defensible. |
| A04 Life-saving crossbow heal — Missing | Associate heal amount/timing, health and subsequent observed damage. Compute a bounded counterfactual health estimate with uncertainty. | Save-like sequence; overheal, another simultaneous healer, missing damage, death despite heal and long delay cases. Never state certain death from speculative future damage. |
| A05 Long-distance airborne/medic arrows — Partial | Distinguish healing bolts from damaging arrows; measure launch-to-impact separation and target movement/class. Combine evidence without duplicate bonuses. | Long/short distance, airborne/grounded, medic/nonmedic, healing/damage and missing projectile cases. |
| A06 Sticky traps — Missing | Associate owned planted stickies, dwell time, detonation cluster and victims. Evaluate medic picks/player advantage separately. | Stationary trap vs sticky spam, simultaneous unrelated detonations, owner change and no kill. Reliable attribution is prerequisite. |
| A07 Airblast pit kills — Missing | Correlate credited push/airblast evidence, displacement and environmental death within a bounded window. | Airblast fall positive; ordinary fall, rocket knockback, reflect projectile kill and ambiguous attribution negatives/unknown. |
| A08 Trimp kills — Partial | Add movement classification before charge-melee/shield kills, preserving existing charge tags. | Actual trimp vs ordinary charge, jump or ramp movement. If slope/contact data is insufficient, label probable rather than confirmed. |
| A09 Medic surf escape — Missing | Track incoming blast, displacement/speed, escape distance, survival and number/timing of attackers. | Four-player sack escape vs ordinary rocket jump, dying after launch and lone unrelated enemy. Travel distance must exclude teleports/respawns. |
| A10 Low-health medic survival — Missing | Build sustained low-health survival episodes with threat and recovery evidence. | Survives pressure vs idling safely at low HP; prevent repeated candidates every tick. Death ends episode. |
| A11 Team-wipe significance — Partial | Separate multi-victim wipe contribution, last-enemy cleanup and team-assisted wipe. Compare scoring alternatives. | Killing one remaining soldier must not earn the same impact as personally removing several living enemies. Account for respawns and roster uncertainty. |
| A12 Longer multi-kill sequences — Partial | Propose inter-kill gap plus total-duration cap; retain rapid bonus and add approved duration penalty. Explicitly define grouping/overlap policy. | 2/3/4+ kills, boundary ticks, long gaps, round changes, victim respawns and duplicate deaths. Compare all old/new clip boundaries; no hidden first-kill-window replacement. |
| A13 Weapon coverage — Partial | Centralize canonical weapon/custom-kill attribution, item-schema aliases, stable fallback and weapon tags; prioritize stickies. | The Original alias, stock/reskins, schema absent, unknown weapon, secondary effects and sticky kills retain truthful weapon data. Do not guess unsupported weapons. |
| A14 Competitive/pub weighting — Missing | Versioned scoring profiles separate from classification; compute alternative scores offline. | Same event detected in both contexts; only approved weights differ. Unknown/PUG confidence has explicit fallback. No automatic old-export rewrite. |
| A15 Competitive mistake analysis — Missing | Define additive candidate kinds for own-team drops, disadvantage, caught-out deaths and other reviewed errors; support POV/STV evidence limits. | Own-team and enemy perspectives distinguished; ordinary death not automatically a mistake. Filters/mode workflow require U approval. |
| A16 events.txt input — Partial | Obtain real file examples, document format/encoding and demo association; parse into normalized bookmarks alongside current JSON sources. | Multiple demos, malformed lines, stale paths, duplicate JSON/text marks and unmatched entries. Normal analysis remains active. |
| A17 Trickstabs — Partial | Preserve confirmed backstab tag; infer trickstab subtype from sufficient movement/facing geometry. | Ordinary backstab negatives and reviewed trickstab examples. Insufficient orientation/state remains unknown. |
| A18 Bleed-out highlights — Missing | Distinguish bleed damage and lethal bleed attribution from direct hit, afterburn and ambiguous delayed damage. | Nonlethal bleed, bleed death, simultaneous damage sources and missing attacker. |
| A19 Reflect kills — Missing | Correlate deflection/ownership transitions or trustworthy custom-kill events with weapon, reflector and victim. | Original projectile owner vs reflector, multiple reflections, airblast pit death and ordinary projectile kill. |

### Feasibility checkpoint

A03/A04/A06/A07/A08/A09/A17 may require evidence the current export does not contain or a demo cannot supply. First deliver a capability matrix by POV/STV and representative demo version. If reliable identification is impossible, ask whether to accept a clearly qualified heuristic, extend parser output, or defer that feature. A heuristic is not a silent substitute for the requested capability.

## 7. Candidates and parsing requirements

| ID / status / gates | Implementation contract | Acceptance criteria |
| --- | --- | --- |
| C01 Row-adjacent Preview — Missing; U/B | Mock up placement, row association and selection semantics while retaining existing Preview functionality. | Approved compact/desktop layout; no lost columns/filter controls; correct demo and padding; no recording triggered. |
| C02 Clear parse inputs after success — Missing; B/U | Clarify clearing current selection/output field versus persisted preferred output directory. Reset only after successful parse, not cancel/failure. | Loaded results and export remain usable; retry state retained after failure; saved defaults preserved unless separately approved. |
| C03 Tag glossary — Missing; U | Generate/reference a centralized registry of emitted tags, meanings, evidence, scope and uncertainty. Choose placement only after mockups. | Every emitted tag resolves; legacy/unknown tags display safely; glossary agrees with detector version. |
| C04 Demo dates and season association — Missing; U/B/D | Define date provenance and timezone; distinguish filesystem dates, filename inference and verified match dates. Add optional match/scrim/season association. | Copying a file does not rewrite an asserted match date; conflicting sources shown as uncertain. External RGL lookup requires separate API/source research and approval of network behavior. |
| C05 Server classification — Partial; B/U | Improve evidence precedence, add PUG identity and calibrated confidence; separate competition format from server/match provenance. | RGL config alone must not prove an official RGL match. Test PUGs using competitive configs, small pubs, offclasses, missing roster and warmup. |
| C06 Manual recording category — Missing; U/D/B | Add recording-kind metadata without overloading automatic batch status; mock up category/filter presentation. | Manual takes and automatic clips distinguished; neither falsely marks the other complete; legacy status handled explicitly. |
| C07 Selection autoscroll — Missing; U/B | Resolve pagination versus scroll interaction before coding. Preserve filtered-selection semantics and modifier behavior. | Drag beyond both edges, wheel while held, release outside, filtered hidden selections, compact layout and keyboard selection. No accidental selection of all demos. |
| C08 Compact class names — Missing; U | Show selected class names using approved wrapping/elision/tooltip behavior without altering map selector. | 0/1/all classes, long names, near 900×530, no overlap or navbar movement. |
| C09 Shortcut columns — Partial; U | Propose uniform columns that fit all shortcuts, with approved grouping and approximately eight entries per column where feasible. | No vertical clipping across approved breakpoints; every bind editable; no binding/default change; guide anchors still correct. |

## 8. Recording and output requirements

| ID / status / gates | Implementation contract | Acceptance criteria |
| --- | --- | --- |
| R01 Batch custom launch options — Partial; U/B | Extend existing validated manual option model to batch only after deciding shared versus separate settings. Preserve managed options and quoting safeguards. | Allowed flags reach TF2; managed options cannot break recording; quoted paths work. `-novid` remains automatically present; improve explanation only with U approval. |
| R02 Short output names + metadata — Partial; B/D/U | Propose human-readable collision-safe filenames and stable JSON identity linkage. Keep original indexing and all media formats. | Duplicate names, reruns, renamed/moved files and legacy long names resolve correctly; replacement becomes visible only after successful finalization. |
| R03 Kill-count category subfolders — Missing; B/D | Define count meaning, e.g. unique victims vs death events, zero-kill candidates and multi-tag category precedence. | Approved hierarchy for 0/1/2/3+; old paths still discoverable; metadata matches media. |
| R04 Simple manual outputs — Missing; B/D/U | Flatten approved manual output hierarchy; retain user ability to choose supported recording formats. Do not remove image sequences to satisfy simpler video organization. | Multiple takes without collision; no unwanted category folders for new manual videos; logs/campaths remain recoverable outside the video browsing path. |
| R05 Emergency batch stop — Missing; U/B | Propose shortcut and stop state machine with graceful flush timeout and recovery. Resolve conflicts with TF2/Director bindings. | Stop during launch, seek, capture, flush and transfer; completed clips retained, unfinished clips never falsely complete; idempotent repeat stop. |
| R06 Multiplayer settings restoration — Partial; B if behavior changes | Inventory settings stored in CFG/custom/video and other actual TF2/Steam locations. Audit batch and manual isolation without assuming CFG backup covers everything. | Before/after file hashes and effective settings match for normal exit, crash, Toolkit close and recovery; backups retained on failure; no silent overwrite of concurrent user changes. |

### Proposed recording lifecycle

Use the current mechanisms where possible. Document transitions equivalent to Prepared → Capturing → Flushing → Finalizing → Verified complete. On interruption transition to Recoverable or Failed, never directly to Complete. Persist enough state before destructive transitions to recover after process termination. UI recorded status must derive from the same validated identity/artifact facts used by skip-recorded logic. A status-cache optimization cannot become a competing authority.

The emergency key must request graceful capture stop first; only escalate process termination under an approved timeout policy. A crash cannot guarantee salvaging an unfinished encoded file. Preserve recoverable artifacts and clearly report limitations rather than claiming all interrupted footage is saved.

## 9. MIRV Director requirements

| ID / status / gates | Implementation contract | Acceptance criteria |
| --- | --- | --- |
| D01 Restart keyframe consistency — Partial; B | Reconcile actual HLAE path state after safe restart/seek with stable snapshots; distinguish stale data from intentionally retained path. | Repeated restart, empty path, delayed print, partial telemetry, load failures and backward seeks; panel and timeline agree with HLAE. |
| D02 Keyframe-only scrolling — Missing; U | Design separate keyframe viewport without displacing other controls or violating approved window layout. | Long path scrolls independently; shortcuts/actions remain reachable at small resolution. |
| D03 All loaded timeline keyframes — Partial; U/B | Propose timeline range/zoom policy for keys outside clip bounds; preserve clip/kill cues. | Before/inside/after-session keys, negative/invalid times and dense overlaps handled; no silent omission. |
| D04 Duplicate-tick overwrite — Missing in Toolkit; B | Verify HLAE same-time behavior first; if necessary replace one exact mapped tick using acknowledged commands and path backup. | One key remains at occupied tick; adjacent ticks unchanged; floating-time rounding and failed commands tested. |
| D05 Drag keyframes — Missing; U/B | Approved drag preview with exact tick, snapping policy, commit/cancel semantics and HLAE time-edit mapping. | Esc cancels; no mutation until commit; collision policy uses D04; out-of-range/zoom/DPI and failed acknowledgments handled. |
| D06 Multi-key edits — Partial; U/B | Preserve selection when applying supported position/angles/FOV/interpolation operations. Define relative versus absolute transforms and exclude moving when requested. | 1/many/all selections; unselected keys unchanged; current single-key edit remains available. Reject invalid values atomically where possible. |
| D07 Delete all keys — Missing; U/B | Approved explicit clear action with confirmation and recoverable path snapshot. | Empty/full path, failed command, reload/undo recovery; display changes only after acknowledgement. |
| D08 Orbit/multi-angle workflow — Partial; U/B/D | Research supported camera/time behavior; propose orbit path builder and reproducible take queue with seed, angle, target and timing metadata. | Predictable orbit, target motion, valid camera positions, repeatable takes, cancellation, output collision handling. Literal frozen-time bullet-time is a separate feasibility gate; do not promise it from ordinary campaths. |

## 10. Existing-feature defect validation backlog

These are not missing-feature claims. Reproduce first; document expected behavior, baseline result and minimized fixture. If a fix changes user-visible semantics rather than restoring an approved contract, obtain B/U approval.

| ID | Report | Required investigation / regression test |
| --- | --- | --- |
| V01 | Repeating clips | Compare candidate identity, overlapping intervals, sequence grouping and recording queue duplication; preserve distinct perspectives. Coordinate A12. |
| V02 | No/skip previously recorded behaves incorrectly | Test mixed recorded/new, all recorded, missing artifact, failed prior output and Cancel; verify selection retained. |
| V03 | Table recorded flag disagrees with recording prompt | Trace index, cache, finalization and refresh events using identical IDs. Confirm table and skip logic converge without restart. |
| V04 | Reloaded exports show wrong recordings | Test legacy/new exports, moved demo/output, alternate output root and identically named different demos; prevent false matches. |
| V05 | Airshot false positives / score | Labeled problematic ticks plus grounded ledges, jumps, splash, direct impact, Loose Cannon self-launch and missing state; preserve existing safeguards. |
| V06 | Hitsounds/config/menu state not restored | Hash/provenance audit and effective-setting comparison for batch/manual normal exit, crash and Toolkit closure; test recovery interruption. |
| V07 | Crash/early exit loses or mislabels captures | Inject failures during every recording lifecycle stage; verify only complete validated clips marked complete and recovery is idempotent. |
| V08 | FPS settings ineffective | Inspect actual produced file frame rate, frame count and duration, plus audio synchronization, for supported formats and representative configured FPS values. Command strings alone are insufficient. |
| V09 | Safe restart/timeline inconsistency | Compare HLAE print/XML data to rendered keyframes before/after restart, import and seeking; coordinate D01/D03. |

## 11. Verification strategy and quality gates

### Baseline capture before implementation

Record current HEAD, toolchain, dependency versions, HLAE/TF2 versions and existing test results. Capture approved reference screenshots using the real app. Inventory existing functions and default settings so new work cannot silently narrow capability.

Build a consented fixture corpus with STV and POV demos, competitive/PUG/pub examples, different maps/classes, bookmarks, legacy exports, missing-schema cases and known reported failures. Keep private demos out of public commits unless expressly authorized. Each label needs demo identity, tick coordinate system, event interval, expected tag and review notes.

### Test layers

1. Pure logic: boundary timing, identity, scoring, event attribution, confidence and malformed input.
2. Export/analyzer integration: real decode → evidence → candidates → serialization → legacy load.
3. UI/controller: unchanged filtering/selection, enabled states, guide anchors and approved responsive behavior.
4. HLAE/TF2 end-to-end: actual commands, settings, video/audio, keyframes and restored original files.
5. Packaging: existing supported platform checks and smoke tests. Unsupported live TF2/HLAE tests must be reported as untested, not passed.

Use tests that detect meaningful failures. Add focused tests alongside each change; do not generate large suites that merely repeat implementation details. Rerun broad checks at integration/release boundaries, not after every prose/layout adjustment.

### Required matrices

- UI: existing breakpoint boundaries plus 800×480, near 900×530, 1100×580, 1320×720 and Candidates' wider size; 1920×1080/2560×1440 displays at representative DPI scaling. Use actual supported minimum sizes if these proposed cases conflict. No clipped controls, nav movement or app-wide scroll. UI modifications require approved mockups before coding.
- Data: old/new export schema, known/unknown weapon/schema, POV/STV, single/multiple demos, missing/delayed events and round boundaries.
- Media: automatic/manual; every supported output family; representative low/high FPS; audio on/off; successful/interrupted/recovered capture.
- Files: Unicode/spaces, duplicate names, existing outputs, moved/deleted media, insufficient disk, permissions and interrupted transfer.
- Config: normal exit, crash, Toolkit close, repeated recovery and externally changed files.

### Definition of done for each requirement

Requirement and approval traceable; implementation complete including negative/error cases; legacy behavior preserved outside approved scope; relevant tests pass; screenshots match approved mockups where applicable; measurements and limitations recorded; migration/rollback rehearsed when relevant; no unrelated deletions/refactors; owner review completed. “Released” additionally requires successful package verification and release evidence. No requirement becomes complete solely because a control exists or a command was generated.

## 12. Dependency-ordered delivery plan

This is sequencing guidance, not permission to start or a delivery-time promise.

| Phase | Work | Exit checkpoint |
| --- | --- | --- |
| 0 — Baseline and proposals | Reconcile latest source; fixture/evidence inventory; approval register; baseline tests and screenshots. | Scope mapped; no undocumented behavior changes. |
| 1 — Reliability | V01–V09 investigations; recording identity/restoration contracts; D01 investigation. | Reproducible defects distinguished from missing data or user-state issues. |
| 2 — Shared evidence | Time/identity abstraction, weapon registry A13, source capability matrix; prototype A16 importer. | Existing output parity in compatibility mode; new evidence demonstrably available. |
| 3 — Detectors | A01–A10 and A17–A19 in isolated increments; A11/A12 comparison proposals. | Labeled evaluation and approved thresholds; unknown states explicit. |
| 4 — Context | C05 classification → A14 scoring profiles; C04 dates; A15 mistake-analysis definitions. | Classification calibrated before changing context-dependent scores. |
| 5 — Output and capture | R01–R06 and C06; integrate validated V02–V08 fixes. | New/legacy identity reconciliation, safe stop and restoration validated. |
| 6 — Director | D02/D03 mockups; D04 semantics → D05 dragging; D06/D07; D08 feasibility then implementation. | Commands verified against actual HLAE; saved paths remain recoverable. |
| 7 — Approved UI and documentation | C01–C03, C07–C09 and approved UI portions from every phase; glossary/onboarding updates. | All presentation changes match approved designs at required sizes. |
| 8 — Release qualification | Full regression corpus, native recording runs, package smoke tests, release notes. | Known limitations explicit; rollback artifacts retained. |

UI phases may move earlier when needed, but U gates never disappear. Backend work cannot silently expose new default scores/tags through the existing UI; that observable behavior still requires the applicable approval.

## 13. Risks, decisions and unresolved questions

| Risk / decision | Required handling |
| --- | --- |
| Saves and tactical mistakes are inferred, not certain | Confidence/evidence wording and minimum capability agreed before implementation. |
| Changed scoring/grouping reshuffles favorite clips | Side-by-side candidate/score comparison and explicit B approval; preserve legacy analyzer version. |
| Schema changes break old exports | Additive fields, versioned readers and legacy fixtures; no destructive reparse. |
| Output simplification breaks recording recognition | Stable identity and legacy path reconciliation before naming rollout. |
| Config recovery overwrites newer personal edits | Detect divergence; retain both original and current versions; ask before overwriting ambiguity. |
| Timeline time differs from HLAE time | Round-trip saved XML/print/tick mapping tests before edit or drag support. |
| Too many detections increase noise/runtime | Per-detector metrics, bounded analysis, representative performance tests and approved thresholds. |
| UI space conflicts with preservation rules | Stop for faithful compact/desktop mockups; do not solve with unapproved layout replacement. |
| RGL date/provenance cannot be reliably inferred | Preserve unknown and manual association; research external data access before proposing integration. |
| Unavailable native Windows/TF2 environment | Complete eligible tests; explicitly leave live behavior unverified until executable test evidence exists. |

Open decisions to resolve at the relevant phase: compact preview placement; reset field versus saved default; sequence gap/duration/deduplication policy; confidence thresholds; count definition for folders; manual versus batch recording status; short filename pattern; emergency key/timeout; timeline extent/collision policy; allowable multi-key transforms; orbit versus frozen-time bullet-time scope; classification overrides and scoring profile activation.

No need to answer these all at once. Prepare concrete alternatives and evidence at the associated gate.

## 14. Templates for future implementation sessions

### Change proposal / ADR

- ID and title:
- Requirement IDs:
- Baseline commit and current behavior:
- Proposed behavior and affected interfaces:
- Alternatives considered and reason for selection:
- Protected features checked:
- UI mockup filenames/versions and sizes, if applicable:
- Data/compatibility/rollback implications:
- Verification plan and acceptance thresholds:
- Exact approval requested:
- Decision, user approval reference, date and remaining conditions:

### Requirement verification record

- Requirement ID / implementation commit / PR:
- Approved proposal reference:
- Positive, negative and missing-data fixtures:
- Automated results and native runtime results:
- Before/after measurements:
- UI evidence, if applicable:
- Legacy compatibility and rollback results:
- Known limitations / unresolved defects:
- State: Implemented / Verified / Released, with evidence:

### Session handoff

Record current branch/HEAD, user approvals, completed IDs, remaining blockers, tests run, files changed and next safe step. Re-read this guideline and the latest approval register after a chat interruption. Never infer that an unapproved proposal became approved during the interruption.

## 15. Initial approval and completion register

**All A01–A19, C01–C09, R01–R06 and D01–D08 remain Planned. All V01–V09 remain Investigation.** Initial status is inherited from the code audit, not an implementation promise. There are 42 feature requirements and 9 validation investigations in this version. No application changes, mockups or behavior changes are approved by creation of this document.

The first future implementation session should reconcile current source, establish fixtures and present the first concrete change proposals. Keep this document versioned and update requirement status only with linked evidence.
