#!/usr/bin/env python3
"""Check reconstructed state parity while ignoring approved protection additions."""
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


def apply(current, row):
    for kind in ("players", "projectiles"):
        for entity in row["removed_" + kind]:
            current[kind].pop(entity, None)
        for entity in row[kind]:
            value = {key: val for key, val in entity.items() if key not in FIELDS}
            current[kind][entity["entity_id"]] = value


def compare(baseline, trial):
    left, right = iter(rows(baseline)), iter(rows(trial))
    a, b = next(left, None), next(right, None)
    states = [{"players": {}, "projectiles": {}} for _ in range(2)]
    counts = {field: Counter() for field in FIELDS}
    packets = 0
    while a is not None or b is not None:
        packet = min(row["packet_sequence"] for row in (a, b) if row is not None)
        if a is not None and a["packet_sequence"] == packet:
            apply(states[0], a)
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
            apply(states[1], b)
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
    args = parser.parse_args()
    print(json.dumps(compare(args.baseline, args.trial), indent=2))
