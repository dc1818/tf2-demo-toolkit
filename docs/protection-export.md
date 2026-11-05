# Optional protection observations — A03-P1

Approved by owner November 5, 2026. Five additive fields in each exported
player snapshot:

- invulnerable
- invulnerability_wearing_off
- uber_bullet_resist
- uber_blast_resist
- uber_fire_resist

Each is boolean or null. Unknown is not false. Older exports omit these
fields and remain valid. No existing field is renamed, removed, or assigned
a new meaning. The existing has_condition API and legacy bitset updates
remain unchanged.

## Provenance

A separate nonserialized six-word observation cache tracks successful wire
decoding: base word, four extended words, and the exclusive first word.
An invalid property value clears that source's observation instead of
manufacturing zero.

For conditions below 32, either source's set bit supports true. False
requires both base and exclusive first-word sources to have been observed
without the bit. Otherwise emit null. Extended-word bits use their observed
source. This deliberately sacrifices some negative coverage rather than
claiming a condition was absent when only one source is known.

An observation is valid only for a live player in PVS. The cache is cleared
on entity entry/reentry, PVS loss, entity removal, and user/Steam identity
replacement. The existing stored legacy conditions are not cleared or
otherwise changed by this mechanism.

Conditions can change while all previously exported fields remain constant.
Such a transition now creates an additional state delta. Existing consumers
must tolerate these extra records. Verification compares reconstructed
legacy state after every packet, rather than demanding identical NDJSON
record counts. Candidate output parity is checked separately.

## Compatibility and tests

The parser/exporter unit tests cover unknown/true/false, independent source
words, legacy API semantics, visibility/death gating, old serialized players
and legacy readers ignoring optional fields. The offline context probe
handles both older exports and the new nullable fields. It records condition
transitions without claiming exact damage prevented or a death prevented.

No automatic candidate tags, scores or UI changes are part of this change.
The owner's arrow-save definition is recorded in arrow-save-definition.md.

## Verification results

Rust 1.88: parser library 10 tests and exporter binary 2 tests passed.
Python context suite: 14 tests passed. A release exporter was built from the
final modified source and run against both existing public fixtures.

| Fixture | State packets compared | Baseline state bytes | New state bytes | New export elapsed |
| --- | ---: | ---: | ---: | ---: |
| gully POV | 102,576 | 325,936,238 | 409,920,939 | 18.02 s |
| comp STV | 85,825 | 552,321,281 | 705,915,638 | 21.70 s |

Reconstructed legacy player and projectile state matched at every packet in
the union of both streams. Both samples contain true/false/null invulnerability
observations. Resistance true cases are covered synthetically, not by a
known resistance deployment in these two demos.

The unchanged baseline analyzer successfully consumed both new exports.
Legacy content of all 11 POV and 56 STV candidates matched after excluding
only the five additive evidence keys and resolving the different relative
source-path spellings to the same absolute demo paths. Candidate NDJSON
bytes are **not identical**: the analyzer retains extra fields inside its
state-evidence objects. Scores, tags, grouping, kill ticks, existing evidence
and candidate ordering are unchanged.

State-stream bytes grew 25.8% and 27.8%, respectively. These are not total
export growth percentages. Export timings are new-build observations, not
controlled old/new performance benchmarks. Existing size/time estimates
have not been recalibrated as part of this narrow approved change.

No Windows TF2/HLAE recording tests were run for this parser-only change.
