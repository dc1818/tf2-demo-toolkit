# Original TF2 profile recovery checks — V06

November 9, 2026. Baseline: `ee9d30e491717df750ba8fe0ce3b2e3fb23a138c`.
Work branch: `work/demo-toolkit-roadmap`, draft PR #62. This is the next scoped
phase 1 reliability repair. It does not close the complete V06/R06 restoration
matrix or change the UI, recording settings, detector behavior or output paths.

## Defect

The existing restore path verifies original CFG/custom fingerprints after moving
the original trees back into TF2. A changed or missing backup, or a user's edit
after an interrupted partial restore, can therefore cause failure only after
another original tree has already moved and the current temporary tree has been
removed. The diagnostic says recovery needs attention, but the pre-failure
filesystem arrangement has already changed.

Three regression tests against unchanged `ee9d30e` reproduced this ordering
problem: changed CFG backup, missing CFG backup and a new edit to CFG after a
partial restoration. All three failed their preservation assertions.

## Repair

Before restoring either folder, and after the existing process-exit and active
session checks, validate the available original sources against the already
stored fingerprints:

- Use `cfg_original` / `custom_original` when their backup entries exist.
- If an original folder existed but its backup has already moved, verify the
  current folder instead. This supports interrupted recovery without assuming
  that an arbitrary current folder is the original.
- Validate both trees before mutating either. Mismatch or read failure retains
  current files, backups and the recovery record and uses existing error handling.
- Compare the separately retained hitsound copies with the verified original
  custom tree before moving it, because the final verifier still requires those
  copies to match.
- Remap the existing temporary-path exclusions into the selected backup/current
  tree. Generated session files do not become false corruption reports.
- When a folder did not originally exist, preserve the existing fresh-install
  cleanup behavior. Older records without fingerprints keep their existing
  recovery path; no new fingerprint is guessed for them.

Final post-restoration verification remains in place. The new step supplements it;
it does not mark restoration complete early or remove the active recovery record
on failure. A valid retry after the discrepancy is resolved follows the same
restore/verify/cleanup flow as before.

## Scope and approval

The owner requested config/hitsound restoration checks and continued implementation
under the technical guide. This repair enforces the existing preservation and
recovery contract. It does not introduce a settings conflict-resolution UI,
choose which divergent user version to keep, or overwrite ambiguity automatically.
Any new UI still requires the owner's permission and approved mockups.

No new persisted fields, output names, backup naming scheme or settings defaults
are introduced. No parser/scoring logic, Slint source, typography, layout, tab
position or guide overlay changes. Tests use disposable fixture directories only.

## Verification

Six new production-code Rust tests cover:

1. Changed CFG backup, changed original hitsound in the custom backup, and changed
   separate hitsound copy: both live trees and all backup bytes remain unchanged
   on failure; correcting the fixture permits successful, repeatable restoration.
2. A missing original CFG backup does not consume the intact custom backup;
   restoring its location permits retry.
3. An edit after partial CFG restoration remains untouched, as does the remaining
   custom backup; resolving the discrepancy permits retry.
4. Valid interrupted restoration resumes with either tree already restored and
   returns exact original tree fingerprints.
5. Generated temporary paths are correctly excluded even inside the backup tree;
   user config is restored and normal temporary-file cleanup remains effective.
6. A recovery record without fingerprints retains its supported restore path.

The baseline failures were measured on the first three tests before the repair.
The later extra corruption/compatibility cases extend coverage of the repair;
they are not additional claimed baseline test runs.

Rust 1.88.0 on Linux, `cargo test --locked --workspace`: **205 tests passed**
(156 application, 33 Director, 14 parser library and 2 exporter). All six new
restoration tests passed. The full native application compiled with the locked
dependencies. `git diff --check` passed. This is local native Linux verification;
new Windows/macOS CI is tracked separately in PR #62. Existing tests also cover batch/manual profile isolation, mastercomfig exclusion, full
original-byte restoration, staging rollback, fresh installs, legacy profile names
and a retry after final verification fails.

## Remaining limits

- The checks add a read of the original CFG/custom trees and hitsound copies
  before restore. They stream hashes rather than loading whole trees into memory,
  but restoration I/O increases; a representative large Windows installation has
  not been benchmarked.
- This is not a lock against external programs modifying files between the
  pre-check and the filesystem operations. The existing final verification remains
  necessary. A full transactional restore/journal design is outside this fix.
- It does not classify intentional changes to the temporary recording profile or
  inventory every setting held outside the backed-up locations, such as external
  Steam state. Those remain part of R06/V06.
- Live TF2 launch/exit/crash, HLAE shutdown, registry restoration on a real Windows
  installation, media recovery and FPS validation remain outstanding. Synthetic
  original-file preservation tests are not substitutes for those workflows.
- V01, V05 and V07–V09 remain open. No duplicate-clip, airshot-score, emergency-stop
  or keyframe behavior is altered by this checkpoint.

Rollback is a source revert on the work branch; no data migration is needed.
Do not merge or push to main without the owner's explicit final permission.
