# Repeated recording queue entries (V01)

November 14, 2026. Baseline commit `992fe133e9e3158ba4dc2c9c096607a5b7a6265a`.
Work branch `work/ralph-recovery-pilot`.

## Reproduction

When an imported candidate export contains identical rows and both rows are
selected, the recording snapshot previously included both rows in the batch.
The scheduler received two requested captures and could allocate distinct
output names for them. A synthetic selection with a duplicate, an overlapping
candidate and a distinct POV returned four queue entries at the baseline.

## Repair

The recording snapshot now keeps the first selected copy of an identical row.
The queue and recording space estimate use the same equality rule, and the
confirmation count follows the deduplicated selection. All row selections
and recorded flags remain available in the table. The check compares full
candidate data within the same source path and candidate ID, so differences in
perspective, tags, evidence, and bounds are preserved even when IDs repeat.
This repair leaves analyzer grouping and persisted recording keys unchanged.

## Verification

The regression passed on Windows with Rust 1.88.0 after the fix. Both
recording choices produced three entries, retained the overlap and separate
POV, and agreed with the space estimate. Both V07-P1 regression tests passed.
The locked Rust workspace run passed 199 tests and hit the same ten existing
Windows sandbox failures documented in [V07-P1](recording-recovery-validation.md).
Live HLAE scheduling and the original user-reported repeat case still need
native verification. Distinct or partially overlapping candidates that
produce visually similar footage remain part of the V01/A12 investigation.
