# Recording recognition and skip-recorded reliability — V02/V03/V04

November 9, 2026. Baseline: `c83b8e0b3a0318eaab89015041049d74c81726ab`.
Work branch: `work/demo-toolkit-roadmap`; draft PR #62. This checkpoint addresses
confirmed defects under the guide's phase 1 reliability work. It is not a new
feature, UI redesign, output migration or permission to merge to main.

## Reproduced defect and repair

The baseline recording-folder scan accepts either a recording-key token or a
legacy candidate-name/tick match. Consequently a filename with demo A's explicit
token can be indexed for demo B if both candidates share the same ID and tick
range. An already persisted false match can then affect the REC column, the
skip-recorded prompt and the files considered for replacement.

The repair makes a recognized generated identity field authoritative. It must
match the expected 24-hex token exactly, with only the existing numeric collision
suffix accepted. A foreign or malformed generated token cannot fall through to
legacy matching. Tokenless legacy names retain their existing discovery path;
indexed renamed files and fingerprint-based relocation remain supported. Existing
false associations with explicit foreign tokens are ignored for status and
replacement; files and index history are not deleted or mass-migrated.

The baseline also inserts an entry in the live index before attempting its disk
write. If persistence fails, that instance claims success that a restarted app
cannot recover. Entries now enter the live map only after the write succeeds.
Relocated entries use the same ordering. A background result can replace an old
foreign-token association, while an existing valid association remains protected.

## Shared controller state

- At Record Selected, recompute indexed artifact existence and build one immutable
  selection snapshot. Update the existing REC display from that snapshot; use it
  for both the prompt count and the Yes/No recording queue.
- Deleted or empty files therefore stop causing selected candidates to be
  skipped merely because the UI cached an earlier true value.
- Yes retains selected candidates; No excludes only those recorded in that
  snapshot; Cancel retains the original selection and starts nothing.
- Preserve filtered/hidden selections and selection order. No new selection,
  deduplication, grouping, scoring or recording identity rules are introduced.
- Run the existing background recording-folder reconciliation after successful
  parsing, as well as when loading an earlier export.
- Do not apply a background result to the visible table if its export or output
  root is no longer current. No recording-root discovery scan is added to the
  prompt; indexed image-sequence existence checks still inspect their own files.

## Preservation and authority

The owner requested investigation/fixes for incorrect skip-recorded behavior,
wrong recorded flags and previously parsed exports recognizing the wrong clips,
and then authorized proceeding under the technical guide. These repairs restore
the existing contract that a recording belongs to its actual demo/candidate and
that Yes/No reflects the same status shown in the table. They do not activate an
unapproved scoring/detector proposal or modify UI presentation.

No UI source, labels, dialog wording, fonts, breakpoints, defaults, output names,
recording formats, index schema, parser/analyzer behavior or camera controls are
changed. Legacy key calculation and candidate identity calculation are unchanged.
No real TF2 config or user media is touched by the tests.

## Verification

Eight new Rust regression tests exercise the production recording/index code:

1. Two different demos with identical candidate IDs/ticks cannot claim the same
   explicitly tokened recording; the correct demo still finds it.
2. Prefix/suffix token lookalikes do not match.
3. Legacy tokenless names, current names, numeric collision suffixes and image
   sequence folders remain discoverable, persist on reload and reconcile idempotently.
4. A failed index write does not publish recorded status or remove output.
5. Old persisted false associations do not become recorded/replacement targets;
   a correct background association replaces the invalid cached association.
6. Mixed/all-recorded selections, deleted/empty media, empty selection and repeated
   Yes/No selection queries preserve user choices and return the expected queue.
7. A copied demo and renamed/moved recording under an alternate output root retain
   their portable identity through fingerprint reconciliation and index reload.
8. Legacy index keys remain readable and their existing outputs discoverable.

Test media bytes are deliberately tiny fixtures. These tests exercise identity,
filesystem existence, persistence and queue selection, not playable-video validity.
On an isolated checkout of the unchanged baseline, the first four tests produced
**three expected failures and one compatibility pass**: foreign-token fallback,
token boundary matching and failed-write publication all failed; legacy/current
output discovery passed. This establishes defects in the old code rather than
only testing the replacement implementation.

Repaired source, Rust 1.88.0 on Linux, with the locked dependency graph:
`cargo test --locked --workspace` passed **199 tests** (150 application,
33 Director, 14 parser library and 2 exporter), including all eight new
regressions. The full native Slint app compiled; this was not a stubbed/headless
replacement of the application modules. Existing Director rendering tests ran,
but no claim is made about live TF2 capture or every app breakpoint.
`python -m unittest discover -s tools/tests -v` passed **18 tests**.
`git diff --check` passed. Windows/macOS CI for this new checkpoint is separate
from the previously successful A03/A04 run.

## Remaining work / honest status

The live fixtures below cover part of the V02/V03/V04 acceptance matrix. The
owner's original demo and output folders, replacement via Yes in the Previously
Recorded Candidates dialog, and filesystem changes after the prompt snapshot
still need live review.

Tokenless legacy filenames can still be ambiguous across demos: preserving their
existing import behavior is intentional here. A migration/attribution proposal
with explicit legacy evidence is needed before replacing that contract. Media
existence is not full container/frame/audio validity; that remains V07/V08.

V01 was inspected but not closed: the existing analyzer supports more than two
kills and groups within a first-kill-based window. No supplied duplicate-clip
fixture was reproduced in this checkpoint. Wider gaps, overlap removal or a
duration penalty would change A12 behavior and remain separately approval-gated.
V05–V09 also remain open; none is claimed fixed by these changes.

Rollback is a source revert on the work branch. No stored schema or output layout
changes require data rollback. Existing user clips remain in their original paths.

## Live Windows mixed queue and export reload

November 26, 2026. Pilot code `3c8d94d`. Ralph reviewed the acceptance fixture
before the run. An offscreen Windows harness invoked the normal export-load,
Record Selected, and recording-completion callbacks. It supplied the folder
choice and dialog responses at the picker and prompt boundaries. Windows TF2
and HLAE performed the actual batch recording in an isolated installation.
[SourceTV demo used for the fixture](https://demos.tf/1530633)

- Loading the fixture marked the existing indexed MP4 as recorded and left
  the new clip and a same-ID, same-tick candidate with a different demo
  fingerprint unrecorded. The negative fixture changed a padding byte in the
  demo header to give the otherwise identical replay a different fingerprint.
- Cancel reported one existing selection and preserved both selected rows
  without launching. No on that mixed selection recorded just the new clip.
  The REC column updated on completion without restarting. A second No with
  both rows now recorded preserved the selections and exited before preflight.
- After moving the indexed MP4 to a supported Videos folder under an alternate
  output root and loading the export with an identical renamed demo copy,
  asynchronous reconciliation restored the REC flag. Reopening the index
  confirmed the relocated path, and the different-fingerprint row stayed
  unrecorded. Both final MP4s matched their index SHA-256 and decoded with audio.
- TF2 configuration and custom file hashes and the display registry snapshot
  matched after exit. The first fixture attempt put the renamed video outside
  Videos. A clean rerun at the supported location passed all checks.

## Native Windows dialog clicks

November 26, 2026. Pilot checkpoint `6db5d73`. The Ralph-reviewed fixture used
a visible Toolkit window and the production recording callbacks and dialogs.
The fixture supplied the export folder and initial selection. The first launch
omitted the recording-ready flag in the harness. The clean rerun initialized
it as the normal app does and used mouse clicks through the native UI.

- **Record Selected → Cancel** showed one previously recorded candidate.
  Both rows stayed selected, the index and existing MP4 were unchanged, and
  no preflight or recording session started.
- **Record Selected → No** opened the HLAE preflight for one clip at 1280×720
  and 120 FPS. Clicking **Yes** in preflight ran TF2 and HLAE to completion.
  The app reported one completed clip and zero failures. The new row changed
  to Recorded while the unrelated third row stayed unrecorded. Both selected
  rows stayed selected.
- The existing MP4 kept its original SHA-256. Both MP4s matched their index
  entries, and FFmpeg decoded both audio and video streams. The isolated TF2
  configuration and custom file hashes and the display registry values matched
  their pre-run snapshots after TF2 and the Toolkit exited.
