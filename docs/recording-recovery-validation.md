# Failed recording finalization during recovery (V07-P1)

November 14, 2026. Baseline commit `d22e20f4783a16271715a81faeac77008ce885bc`.
Pilot branch `work/ralph-recovery-pilot`.

## Reproduction

A retained session can have both source capture artifacts and a nonempty
destination. When finalization errored at the baseline, recovery still used
that destination. It wrote the fallback path and fingerprint to the manifest
and updated the recording index while the clip stayed pending. With
replacement enabled it also deleted the previously indexed output.

Both regression tests failed before the repair. An isolated TGA fixture supplied
a nonempty capture frame and fallback file and omitted the Frames destination
to trigger the finalizer error. One test started with no index and observed the
fallback entry. The other started with an indexed recording and observed its
replacement by the fallback. Inspection of the failed fixture confirmed the
prior recording was deleted.

## Repair and verification

On a finalization error, recovery logs and reports the error, marks the clip
`RecoveryPending` and proceeds to the next clip before indexing or removing
any output. The manifest keeps any previously recorded output fields. Successful
finalization and the existing fallback when source artifacts are absent follow
their previous paths. The private recovery helper accepts a fixture-specific
index path. Production supplies the standard index path and uses the same
index size and entry limits.

- Both regression tests passed on Windows with Rust 1.88.0. They each run
  recovery twice and check the diagnostic, pending state, unchanged index,
  retained session, frame and fallback, and retained prior recording where
  applicable.
- The locked Windows Rust workspace run passed 198 tests. The same ten existing
  tests failed on the baseline logic and the repaired logic in the desktop
  sandbox. One hit an access-denied error creating a hard link. The other nine
  depend on the TF2 process check, and the sandbox's `tasklist` call returned an
  error instead of process state.
- The Python suite had 17 passes and one existing failure on both runs.
  `test_optional_values_remain_distinct_and_typed` hit a Windows sharing
  violation during temporary-file cleanup.

The fixtures exercise recovery ordering with synthetic bytes. Live Windows
TF2/HLAE verification and the broader V07 crash-stage matrix remain open.
Finalizer side effects before an error and indexes written by older versions
are outside the scope of this repair.

## Image capture artifact discovery (V07-P2)

Baseline `19de8d3b1ca65f6ec96bf99c5d0042a06a709ef8`. The image artifact
scan treated an empty capture prefix as a match for every nonempty file in
the game directory. The finalizer also attempted to move arbitrary entries
sharing the prefix, including directories and text files.

The batch scan and image finalizer now share a predicate. It requires a known
capture prefix and a nonempty regular TGA, JPEG or WAV file. A legacy entry
with no capture prefix can still use the existing completed-output fallback.
Frame and audio paths and the output index format are unchanged.

Two regression tests reproduced the baseline failures. One verifies that
the finalizer transfers a frame and its WAV while retaining a similarly named
text file and directory. The other verifies that an unidentifiable legacy
entry leaves a different capture's frame alone. Both use synthetic files.
Both passed after the repair on Windows with Rust 1.88.0. The locked workspace
run passed 201 tests and reproduced the same ten Windows sandbox failures
listed above.

### Live Windows image capture and restart recovery

November 26, 2026. Pilot source `1fd8fb7`. Ralph reviewed the recovery
path. The fixture ran production capture and restart recovery against a public
SourceTV demo in an isolated TF2 installation. [SourceTV demo 1530633](https://demos.tf/1530633)

- A normal JPG batch completed with 180 contiguous and decodable 1280×720
  frames at the 60 FPS capture setting. The Toolkit indexed the sequence.
  This JPEG run emitted no WAV, so the interruption fixture used TGA.
- The TGA batch ran three clips. The fixture held the Toolkit monitor before
  finalization, let TF2 finish the first clip, and stopped the isolated
  processes after the second clip wrote frames and before its stop marker.
  The third clip had never started. Production profile and session recovery
  ran on two subsequent launches.
- Restart recovery put the complete sequence in its expected `Frames` and
  `Audio` directories. Its 180 contiguous TGA frames decoded at 1280×720,
  and its 2.995-second stereo PCM WAV decoded at 44.1 kHz. The recovered
  frame and WAV hashes matched the original artifacts, and the manifest and
  index agreed on the output path and fingerprint.
- The similarly prefixed directory, empty TGA, and text file were left in
  place, as was a frame from a different capture prefix. The unstarted clip
  remained `RecoveryPending` with no media output or index entry. The first
  recovery restored the isolated TF2 cfg/custom file hashes and display
  registry snapshot. The second recovery preserved the media hashes and
  index bytes without adding rows.
- The interrupted clip had only 13 frames and an approximately 0.21-second
  WAV that FFmpeg rejected as corrupt on strict decoding. Recovery moved
  those artifacts, set the manifest to `Completed`, and indexed the clip as
  Recorded. That status persisted on the second launch. This baseline
  accepted nonempty matching frames before establishing that the clip had
  finished. The [v1 follow-up](image-sequence-completion-proposal.md#implementation-checks)
  records verification of the automatic JPG and TGA fix.

## Encoded batch transfer (V07-P3)

Baseline `3bf4fea21167186533c5b1bec1c9a163c371ad25`. When rename
failed, such as for a different output volume, the batch finalizer copied
muxed video directly to the final path. A fault-injected copy reproduced two
failures: a new partial file looked like a recording, and an existing
destination was overwritten with partial bytes.

The copy fallback now writes to a unique staging directory on the destination
volume. It flushes the staged file before renaming it to the final path and
deletes the source only after that rename. Ordinary errors clean up the
staging directory. Abrupt termination can leave a staging directory, whose
contents are outside the recording scan. Source cleanup can be retried after
publication. Same-volume moves, manual captures, and image transfers retain
their existing paths.

Both regressions passed after the repair. Each exercises two interrupted
copies followed by a successful retry. On Windows with Rust 1.88.0, the
locked workspace suite passed all 213 tests. The Python suite passed 17 tests
and reproduced the existing Windows temporary-file cleanup failure. The
transfer tests use synthetic files and fault injection. A later
[live cross-volume test](recording-live-validation.md#interrupted-transfer-recovery-v07-p3)
verified the Windows batch MP4 path. Assessment of files already left at final
paths by earlier builds remains open.
