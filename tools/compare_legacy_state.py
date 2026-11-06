#!/usr/bin/env python3
"""Check reconstructed state parity while ignoring approved additive evidence fields."""
import argparse
from collections import Counter
import json
from pathlib import Path

FIELDS = ("invulnerable", "invulnerability_wearing_off", "uber_bullet_resist",
          "uber_blast_resist", "uber_fire_resist")


def rows(path):
    with path.open() as stream:
        previous = -1
        for line in stream:
            row = json.loads(line)
            packet = row["packet_sequence"]
            if packet <= previous:
                raise ValueError("state rows must have strictly increasing packet order")
            previous = packet
            yield row


UBER_FIELDS = ("damage_blocked_total", "medigun_target")


def apply(current, row, ignored_fields=FIELDS):
    for kind in ("players", "projectiles"):
        for entity in row["removed_" + kind]:
            current[kind].pop(entity, None)
        for entity in row[kind]:
            value = {key: val for key, val in entity.items() if key not in ignored_fields}
            current[kind][entity["entity_id"]] = value


def compare(baseline, trial, uber_evidence=False):
    ignored_fields = FIELDS + UBER_FIELDS if uber_evidence else FIELDS
    left, right = iter(rows(baseline)), iter(rows(trial))
    a, b = next(left, None), next(right, None)
    states = [{"players": {}, "projectiles": {}} for _ in range(2)]
    counts = {field: Counter() for field in FIELDS}
    packets = 0
    while a is not None or b is not None:
        packet = min(row["packet_sequence"] for row in (a, b) if row is not None)
        if a is not None and a["packet_sequence"] == packet:
            apply(states[0], a, ignored_fields)
            a = next(left, None)
        if b is not None and b["packet_sequence"] == packet:
            for player in b["players"]:
                for field in FIELDS:
                    if field not in player:
                        raise ValueError(f"missing {field} at packet {packet}")
                    value = player[field]
                    if value is not None and type(value) is not bool:
                        raise ValueError(f"invalid {field}")
                    counts[field][str(value)] += 1
                if uber_evidence:
                    for field in UBER_FIELDS:
                        if field not in player:
                            raise ValueError(f"missing {field} at packet {packet}")
                    value = player["damage_blocked_total"]
                    if value is not None and (type(value) is not int or not 0 <= value <= 2**32-1):
                        raise ValueError("invalid damage_blocked_total")
                    target = player["medigun_target"]
                    if target is not None:
                        if not isinstance(target, dict) or "entity_id" not in target:
                            raise ValueError("invalid medigun_target")
                        entity = target["entity_id"]
                        if entity is not None and (type(entity) is not int or not 0 < entity <= 2**32-1):
                            raise ValueError("invalid medigun_target entity_id")
            apply(states[1], b, ignored_fields)
            b = next(right, None)
        if states[0] != states[1]:
            raise ValueError(f"legacy state differs after packet {packet}")
        packets += 1
    return {"legacy_state_parity": True, "state_packets_compared": packets,
            "protection_observations": counts}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("trial", type=Path)
    parser.add_argument("--uber-evidence", action="store_true", help="validate and exclude the two A03-P2 additions")
    args = parser.parse_args()
    print(json.dumps(compare(args.baseline, args.trial, args.uber_evidence), indent=2))
