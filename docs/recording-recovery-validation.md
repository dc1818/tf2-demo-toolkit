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
