# Demo Toolkit implementation and guideline audit

Audit date: November 9, 2026. Reviewed branch: `work/demo-toolkit-roadmap`, commit `c83b8e0b3a0318eaab89015041049d74c81726ab`. Baseline/main: `568e2434c4aa04367cc6b767f2f0cabf3f7e62aa`. This is an audit report, not an implementation change or approval to change behavior.

## Conclusion

The application contains substantial earlier functionality, but the technical roadmap is not complete. Of its 42 feature requirements, 2 are newly implemented in the native pipeline with automated checks, 16 remain partial, and 24 remain absent as complete native implementations. Two of those 24, A01 and A02, have earlier reported offline prototype work; they must not be described as untouched. The other 22 have no complete implementation identified in the inspected source. Nine separate defect investigations remain open.

The work branch follows the central preservation rules: it remains an unmerged draft, has no UI-source changes, and adds the approved Medic-save detectors without replacing the prior candidate scoring/grouping path. It has not met the guide's full verification/release definition of done. Documentation, performance qualification, native TF2/HLAE testing and the remaining roadmap work are outstanding.

## Evidence and limits

- [Draft PR #62](https://github.com/dc1818/tf2-demo-toolkit/pull/62) is open and unmerged. Its head and baseline were checked during this audit.
- [CI run 42142900112](https://github.com/dc1818/tf2-demo-toolkit/actions/runs/42142900112) matches the audited head. Workspace checks/tests succeeded on Windows, Linux and macOS; Linux responsive screenshot generation and the optimized Windows build succeeded.
- Reviewed the current guideline, source, baseline-to-head diff, validation documents and earlier chat reports. The diff contains 19 files: detector/parser code, documentation and Python evidence tools/tests. No app or Director UI file, recording module, scheduler, dependency lockfile or packaging file changed on this branch.
- Reran `python -m unittest discover -s tools/tests -v`: **18 passed**. Full Rust/GUI tests were not rerun locally in this audit; their result comes from the matching CI run.
- Prior real-demo parity and timing results below come from committed validation records, not fresh fixture runs in this audit. Existing-candidate parity was reported for 68 candidates after A04, with A03 adding one STV candidate.
- Did not run the user's installed executable, live TF2/HLAE recording, media FPS inspection, config restoration against a real installation, or a fresh visual review at all resolutions. Source presence and screenshot generation are not proof that these workflows are defect-free.

## Earlier requests already represented in the program

These are implemented source paths, not a claim of fresh end-to-end verification of each item.

| Request | Source evidence / qualification |
| --- | --- |
| Toolkit and Director in the same executable | `app/src/main.rs` handles `--director`; `recording.rs` launches the current executable in that mode. |
| TF2 theme, font and prominent Demo Toolkit title | `app/ui/app.slint` and `tf2-theme.slint` use the TF2 theme and TF2 Build font; header says Demo Toolkit. Exact agreement with historical images cannot be established from unavailable images. |
| Three-step Parse and guided Recording Settings | Parse step navigation/gates and Recording Settings sections are present in `app.slint`; `main.rs` validates transitions. |
| Stable navigation while resizing | Header remains at the top and page content begins beneath it. The audited main baseline is the stationary-navigation fix. Fresh native DPI/resize verification remains. |
| Folder drag/drop, remembered demo picker and default C: TF2 demos folder | `main.rs` handles dropped directories and `.dem` files; picker prefers the remembered existing directory, then standard Windows TF2 demo paths. |
| Numbered, scrollable selected-demo lists | Shared `DemoList` component in `app.slint` uses a scroll view; Parse selection/review reuse it. |
| Optional automatic schema detection, validation and help | `main.rs` discovers the schema; `item_schema.rs` validates file type and KeyValues structure. Choose file and How to find are hidden when valid. This follows the later request replacing the earlier Automatic button design. |
| Normal performance default and dynamic resource use | UI displays Normal while retaining legacy `Medium` internally; launch/demo-selection paths reset to Medium. `scheduler.rs` scales worker and CPU limits for Low/Normal/High using available resources and jobs. |
| Parse time/space estimates and disk preflight | `batch.rs` and `preflight.rs` estimate work, calibrate timing and enforce required free space. Regression test `same_demo_estimate_does_not_reverse_modes_with_different_history` exists. Accurate estimates for newly added detectors still need qualification. |
| Recognizable parse export folders and optional grouping | `parse_exports.rs` provides Demo Toolkit Exports and DemoToolkit_Parse_ naming, saved preference handling and picker resolution; the UI explains which previous-export folder to select. |
| Comma-separated tag search | `matches_tag_query` requires every comma-separated phrase to match a tag; earlier space-separated search is preserved. |
| Candidates/Recording Settings gated on results | Header navigation uses `has-export`. |
| Record Selected explanation, Preview and Details selection rules | Record Selected tooltip says it uses Recording Settings; Preview and Details require exactly one selected candidate. |
| Tooltips, disable switch, Options and application logs | Options contains guidance, tooltip preference and log actions. See the compact scrolling mismatch below. |
| First-use guide starting at Parse | `guide_ui.rs` starts at step 1, selects its Parse page, waits for parse results, continues through candidates/settings/actions, and persists onboarding progress. The user's local first-launch preference was not inspected or reset. |
| Recording padding explanation, per-tick tags and tag-category outputs | Padding explains seconds before/after ticks; per-tick and whole-candidate tag groups are preserved; encoded output categories use the primary tag. Kill-count subfolders are a separate unfinished request. |
| Filtered Select All, recording recognition and restore machinery | Controller selection uses filtered visible indices. Recording index, reconciliation, backup/recovery and restore tests exist. Reported defects are still investigations, not proof these features are absent. |
| Voice muting, manual HUD-off, TF2 Quit and campath loading | Existing recording/Director paths support these features. Real capture/restore behavior still requires native testing. |

## Complete roadmap status

“Partial” means related functionality exists but does not fulfill the entire requested contract. “Missing” refers to the requested complete feature, not absence of all supporting code. None of these statuses claims a newly published release.

| ID | Requirement | Audit status and remaining work |
| --- | --- | --- |
| A01 | High-velocity kills | Earlier offline prototype reported; not integrated in native analyzer. Chat reported 10 synthetic tests and 795 kills across four demos, with 2 flagged. Original prototype source was not recovered; results not rerun here. |
| A02 | Ubersaw charge gains | Earlier offline probe reported; native analyzer only recognizes Ubersaw as melee. Chat reported 11 tests, four charge-gain cases and two ambiguous saw cases. Nonlethal attribution and native integration remain. |
| A03 | Life-saving Über with outcome | Implemented in `analyzer/uber_saves.rs` and called during normal analysis. Supported stock-Über evidence only; ambiguity suppresses output. Owner playback/live capture review remains. |
| A04 | Life-saving crossbow heal | Implemented in `analyzer/arrow_saves.rs` and called during normal analysis. Tests include otherwise-lethal afterburn/projectile damage. Conservative handling of other healing/protection/missing evidence; owner playback review remains. |
| A05 | Long-distance airborne/Medic arrows | Partial: projectile/airborne/class information exists; full healing-versus-damage arrow detector and distance validation remain. |
| A06 | Sticky traps and valuable picks | Missing: sticky weapon/projectile recognition does not establish a planted trap, dwell time and detonation attribution. |
| A07 | Airblast into pits | Missing: no complete airblast-to-environmental-death attribution detector identified. |
| A08 | Trimp kills | Partial: charge/shield/melee tags exist; no complete trimp movement classifier. |
| A09 | Medic surfing escape | Missing: no full blast/escape-distance/survival/multiple-attacker episode detector. |
| A10 | Low-health Medic survival | Missing: no sustained pressured-survival episode detector. |
| A11 | Team-wipe significance | Partial: wipe and last-enemy tags exist; last-enemy cleanup still receives the same wipe bonus in the inspected scoring block. |
| A12 | Longer multikills and duration penalty | Partial: sequences can contain more than two kills, but grouping uses the first kill within `SEQUENCE_GAP = 267`; proposed wider inter-kill policy and duration penalty are not implemented. |
| A13 | Comprehensive weapon attribution | Partial: many aliases and projectile/melee types exist; complete canonical attribution/coverage, including problematic sticky cases, is unfinished. |
| A14 | Competitive/pub scoring profiles | Missing: classification does not provide the requested separate versioned scoring profiles. |
| A15 | Competitive mistake analysis | Missing: no separate requested analysis workflow for drops/caught-out/game-changing errors. |
| A16 | events.txt input | Partial: JSON bookmark sources work; no native `events.txt` importer identified. |
| A17 | Trickstabs | Partial: confirmed backstabs exist; movement/facing-based trickstab subtype attribution is absent. |
| A18 | Bleed-out highlights | Missing: no complete bleed-specific lethal attribution detector. |
| A19 | Reflect kills | Missing: no complete reflector/ownership-transition candidate detector. |
| C01 | Row-adjacent Preview / first-column change | Missing: Preview remains a toolbar action; requested placement/removal needs approved mockups. |
| C02 | Clear parse inputs/output after success | Missing: success loads results and navigates to Candidates without clearing demo selection/output fields. Persisted output preference must remain a separate decision. |
| C03 | Tag glossary | Missing: friendly tag display exists, but no complete glossary UI/registry. |
| C04 | Demo dates and season association | Missing: no complete dated provenance/RGL/scrim association feature. |
| C05 | Accurate RGL/PUG/pub classification | Partial: current classifier uses config signatures and roster samples; RGL config can still yield an RGL label. PUG provenance/calibration remains. |
| C06 | Separate manual-recording category | Missing: requested Candidates distinction/filter is absent. |
| C07 | Selection drag autoscroll | Missing: no complete edge-autoscroll interaction identified; filtered Select All is already separate existing functionality. |
| C08 | Compact selected class names | Missing: shared `filter_summary` returns a count for multiple classes, e.g. “4 classes selected.” |
| C09 | Uniform, unclipped MIRV shortcut columns | Partial: shortcut controls exist; requested column distribution and complete breakpoint acceptance are not established. |
| R01 | Custom launch options for batch | Partial: manual option validation/injection exists; batch launch does not consume it. `-novid` is already inserted automatically and rejected as a managed override. |
| R02 | Short output filenames plus metadata | Partial: identity/index/metadata and collision handling exist, but filenames still include demo, candidate, ticks and recording token. |
| R03 | Kill-count subfolders | Missing: primary-tag categories exist, but no requested per-category kill-count hierarchy. |
| R04 | Simple manual output folders | Missing: requested flattening is not implemented; existing format choices must be preserved. |
| R05 | Emergency batch-stop shortcut | Missing: shutdown/recovery machinery exists, but no complete requested shortcut and validated graceful-stop lifecycle. |
| R06 | All multiplayer settings restored | Partial: CFG/custom/hitsound/DX backup and restoration exist; full effective-settings inventory and normal/crash/close validation remain. |
| D01 | Restart keyframe consistency | Partial: synchronization/retry/snapshot gating exists; reported restart defect needs native reproduction. |
| D02 | Separate keyframe-only scrolling | Missing: keyframes share the broader Director controls scroll area. |
| D03 | All loaded keys on timeline | Partial: keys are explicitly filtered to session start/end before supplying both panel and timeline models. |
| D04 | Overwrite duplicate-tick keyframe | Missing as a Toolkit-managed contract. Underlying HLAE behavior must be verified before claiming this works or adding replacement logic. |
| D05 | Drag keys with exact tick | Missing: activation/seeking and command edits exist, but no complete timeline drag workflow. |
| D06 | Multi-key edits | Partial: selection actions and some whole-path commands exist; normal edit builds a single-key selection, so prior multi-selection is not preserved for the requested bulk edits. |
| D07 | Delete all keys | Missing: single-key deletion is present; requested recoverable clear-all operation is absent. |
| D08 | Orbit/matrix/multiple-angle workflow | Partial foundation: campaths and manual recording exist; no complete orbit builder/repeatable multi-take workflow. Frozen-time feasibility remains separate. |

Counts: A = 19, C = 9, R = 6, D = 8, total **42**. Native implementation states: **2 implemented + 16 partial + 24 missing complete implementations**. Historical context subdivides the 24 into **2 reported offline prototypes + 22 other missing features**.

## Existing-feature defects still requiring validation

| ID | Investigation | Evidence status |
| --- | --- | --- |
| V01 | Repeating clips | Grouping/index code exists; reported duplicate scenario not reproduced here. |
| V02 | No/skip previously recorded | “No” filters selected rows using recorded flags; no native mixed-queue reproduction in this audit. |
| V03 | Table/prompt recorded mismatch | Both use state/index machinery and refresh paths; no proof the reported stale-state case is fixed. |
| V04 | Wrong recordings after export reload | Index and reconciliation paths exist; moved/legacy/alternate-root cases remain unverified. |
| V05 | False airshots/ratings | Existing geometric safeguards and tests are present; user's problematic ticks need labeled validation. |
| V06 | Hitsound/config/menu restoration | Restore logic and synthetic file tests exist; real installation lifecycle/effective-state checks remain. |
| V07 | Crash/early-exit capture recovery | Recovery exists; full launch/capture/flush/transfer failure matrix remains. |
| V08 | Actual recording FPS | Settings/commands exist; produced media FPS, duration and audio synchronization were not measured. |
| V09 | Safe restart/timeline inconsistency | Related code exists; live HLAE telemetry versus displayed keys remains unverified, with D03's bounds filter confirmed in source. |

## Technical-guide compliance findings

1. **Branch and preservation rules followed in the inspected diff.** Draft work only; no merge to main. No UI, recording-output layout or existing score/grouping edits. A03/A04 are appended through the existing candidate pipeline; parser observations are additive. Existing data-load paths remain unchanged.
2. **Feature approvals are recorded.** Section 15 and A03/A04 documents record the user's enablement/integration approvals. This audit grants no new B/U/D approvals and makes no UI changes.
3. **Implementation is not full verification.** The guide requires owner review, representative native runtime and relevant media/config tests. Those are still outstanding, so the project cannot be described as fully verified or release-ready.
4. **Historical status needs clearer wording.** A01/A02 table rows say Missing, while section 15 acknowledges offline prototype work. The accurate combined description is “earlier prototype work reported; recovery/revalidation and native integration pending.” The original code is not available in this checkout.
5. **CI documentation is stale.** The PR text says the Windows optimized build is still running, although the matching workflow is now successful. Per-feature docs still emphasize local headless evidence without recording the completed native CI result. The guide asks for these results at checkpoints.
6. **Performance acceptance remains open.** A03 validation records analysis time increasing from 29.087 to 46.587 seconds for STV (~60%) and 11.756 to 17.736 seconds for POV (~51%). State-stream size increased 7.84% and 7.60%. These are single-run observations, not controlled benchmarks. Measurements were disclosed, but approved performance budgets, broader calibration and first-run estimate accuracy are not established.
7. **The existing Options layout conflicts with the earlier no-page-scrolling preference.** `OptionsPage` uses a ScrollView with a compact viewport height of 700 × scale. This predates the roadmap diff, so it is not a new unauthorized UI change; it remains an unresolved product requirement. Any repair requires the requested mockup approval.
8. **Detector evidence is conservative but not exhaustive.** A03 requires supported stock-Über observations and follow-up; A04 rejects ambiguous competing healing/state changes. Positive real-fixture coverage is limited, particularly for POV. No independent visual ground-truth evaluation or precision/recall corpus is recorded. These are documented coverage limits, not proof of general detection accuracy.
9. **Delivery sequencing is incomplete.** Work progressed to approved Medic detectors while V01–V09 reliability investigations remain open. The guide calls its phases sequencing guidance; this is not evidence of unauthorized work, but reliability/release gates cannot be skipped when declaring completion.

## Recommended next checkpoint

Reconcile the progress/CI documentation, recover or explicitly reconstruct and revalidate A01/A02, and retain all nine reliability investigations as open until evidence closes them. Before further detector activation, follow the applicable behavior approval gate. Before any layout correction, present the requested actual-app mockups. Continue only on the work branch; no main merge or release authorization is implied.

No application code, UI, settings, exports, release or remote branch was changed during this audit. This report preserves the pre-implementation audit snapshot. Later V02/V03/V04 fixes and their evidence are recorded separately in [recording status validation](recording-status-validation.md).
