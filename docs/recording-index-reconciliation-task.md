# V04 follow-up for unresolved recording paths

**Next Ralph task.** Review and reproduce recording-index reconciliation against
pilot checkpoint `d79b0d8`. Run the cases below in a disposable fixture.

## Evidence

- A read-only audit found 124 clips in the configured Videos folders. Of 242
  latest recording keys, 109 referenced existing paths and 133 referenced
  paths that did not resolve. The count of 178 also included paths referenced by
  historical index rows.
- Comparing all 124 files against the unresolved entries found no exact
  SHA-256 or generated recording-key token match. Two unresolved entries had
  no saved fingerprint. The audit covered the configured video library.
- The recorded-status lookup checks that an indexed output still exists.
  Candidate selection and replacement lookup use that same check. Export load
  separately tries to reconcile recordings in the configured output root.
- The initial Ralph source review identified two disk-write cases to
  reproduce. Fingerprint relocation can append a byte-identical output with
  another recording's generated token, even though the status lookup later
  rejects that token. A background reconciliation can also append from its
  snapshot before the UI checks that its export and output root are current.

## Acceptance checks

1. Load candidates with one valid indexed clip and one unavailable indexed clip.
   Verify the Recorded column, the prompt count, and the queue after selecting No.
   Verify that the valid clip stays in its category folder and the unavailable
   path is ineligible for replacement.
2. Move and rename a disposable clip within the supported output layout, then
   load the export. Verify that a unique exact match restores the Recorded
   state and persists the relocated path. Reopen the index and repeat the
   load to check that reconciliation does not add redundant rows.
3. Give a different recording a foreign generated token and byte-identical
   media. Confirm that its path is never persisted for the missing original.
   Pause reconciliation after its index snapshot, persist a newer valid
   entry, then resume it. Confirm that a fresh reload still resolves to the
   newer entry. Use index contents on disk for both assertions.
4. Repeat with an unavailable output drive, a later reconnect, a same-name
   different clip, and a file that disappears while the confirmation dialog is
   open. Check that an unresolved or ambiguous entry never skips the wrong
   candidate or authorizes a replacement. Verify recording history and the
   original fixture output paths and hashes after the run.
5. If a fixture reproduces a defect, record the failing case and the smallest
   compatible fix in the implementation register. A change to association
   rules, on-disk history, output layout, or UI follows the guide's approval
   protocol.
