"""Measure a trial per-projectile A06 tag against existing frag candidates."""

import argparse
from collections import Counter
from decimal import Decimal
import json
from pathlib import Path


TAG = "planted_sticky_kill"
THRESHOLDS = ("0.5", "1", "2", "3", "5")
WEAPON_SLOTS = ("primary_handle", "secondary_handle", "melee_handle")
DWELL_BREAKS = (
    "position", "thrower_handle", "owner_handle", "launcher_handle",
    "original_launcher_handle", "deflect_owner_handle", "deflection_count",
)


def rows(path):
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            if line.strip():
                yield json.loads(line)


def event_key(row):
    return row["packet_sequence"], row["event_index_in_packet"]


def remote_kill(event):
    return event["weapon"] == "tf_projectile_pipe_remote" and (
        event["attacker"] > 0 and event["user_id"] > 0
        and event["attacker"] != event["user_id"]
    )


def observe_entity(active, row):
    entity = row["entity_id"]
    if not row["in_pvs"] or row["action"] in ("leave", "delete", "removed", "replaced"):
        active.pop(entity, None)
        return
    track = active.get(entity)
    if (track is None or row["action"] == "enter" or track["serial"] != row["serial"]
            or track["kind"] != row["entity_kind"]):
        track = {"serial": row["serial"], "kind": row["entity_kind"], "fields": {}, "planted": None}
        active[entity] = track
    track["user"] = row["user_id"]
    fields = track["fields"]
    observations = {o["field"]: o["value"] for o in row["observations"]}
    if any(key in observations and key in fields and observations[key] != fields[key]
           for key in DWELL_BREAKS):
        track["planted"] = None
    if "touched" in observations:
        if observations["touched"] != 1:
            track["planted"] = None
    fields.update(observations)
    if (observations.get("touched") == 1 and track["planted"] is None
            and all(fields.get(key) for key in ("position", "original_launcher_handle", "thrower_handle"))):
        track["planted"] = row["server_tick"]


def attribution(active, event, tick):
    entity = event["inflictor_ent_index"]
    bomb = active.get(entity)
    if not bomb or bomb["kind"] != "pipebomb" or bomb["serial"] is None:
        return {"status": "identity_unknown"}
    fields = bomb["fields"]
    detail = {"entity": entity, "serial": bomb["serial"]}
    if fields.get("pipe_mode") != 1:
        return {**detail, "status": "mode_unknown" if fields.get("pipe_mode") is None else "other_mode"}
    thrower = fields.get("thrower_handle")
    launcher = fields.get("original_launcher_handle")
    player = active.get(thrower & 2047) if isinstance(thrower, int) and 0 < thrower < 2097151 else None
    inventories = [
        p["user"] for p in active.values() if p["kind"] == "player"
        and launcher and any(p["fields"].get(slot) == launcher for slot in WEAPON_SLOTS)
    ]
    if (not player or player["kind"] != "player" or player["serial"] != (thrower >> 11)
            or player["user"] != event["attacker"] or inventories != [event["attacker"]]
            or launcher != fields.get("launcher_handle")):
        return {**detail, "status": "ownership_unresolved"}
    planted = bomb["planted"]
    if (tick is None or planted is None or planted > tick or not fields.get("position")):
        return {**detail, "status": "dwell_unknown"}
    return {**detail, "status": "observed", "dwell_ticks": tick - planted}


def analyze(export, candidates_path):
    manifest = json.loads((export / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("sticky_evidence_format_version") != 1:
        raise ValueError("This prototype requires sticky evidence format 1.")
    interval = Decimal(str(manifest["server_tick_interval"])).quantize(Decimal("0.000000001"))
    if interval <= 0:
        raise ValueError("A positive server tick interval is required.")
    events = {event_key(row): row for row in rows(export / "events.ndjson")
              if row["event_type"] == "player_death" and remote_kill(row["event"])}
    evidence = {}
    active = {}
    sidecar = export / manifest["files"]["sticky_evidence"]
    for row in rows(sidecar):
        if row["record_type"] == "entity":
            observe_entity(active, row)
        elif row["event_type"] in ("round_start", "teamplay_round_start"):
            active = {index: track for index, track in active.items() if track["kind"] == "player"}
        elif row["event_type"] == "player_death" and (key := event_key(row)) in events:
            recorded = events[key]
            if row["server_tick"] != recorded["server_tick"]:
                raise ValueError(f"Mismatched death timestamp at {key}.")
            event = recorded["event"]
            evidence[key] = {
                **attribution(active, event, row["server_tick"]),
                "attacker_user_id": event["attacker"], "victim_user_id": event["user_id"],
                "demo_tick": recorded["demo_tick"], "server_tick": row["server_tick"],
            }
    candidates = list(rows(candidates_path))
    result = {
        "baseline_clips": len(candidates),
        "recorded_remote_deaths": len(events),
        "evidence_status_counts": dict(Counter(e["status"] for e in evidence.values())),
        "unjoined_event_count": len(events.keys() - evidence.keys()),
        "death_evidence": [{"packet_sequence": key[0], "event_index_in_packet": key[1], **detail}
                           for key, detail in evidence.items()],
        "thresholds": {},
    }
    for threshold in THRESHOLDS:
        qualified = {key: detail for key, detail in evidence.items() if detail["status"] == "observed"
                     and detail["dwell_ticks"] * interval >= Decimal(threshold)}
        clips = []
        for candidate in candidates:
            matches = {}
            for kill in candidate["kills"]:
                key = event_key(kill)
                detail = qualified.get(key)
                if detail and (detail["attacker_user_id"], detail["victim_user_id"], detail["demo_tick"]) == (
                    kill["attacker_user_id"], kill["victim_user_id"], kill["demo_tick"]
                ):
                    matches[key] = {"packet_sequence": key[0], "event_index_in_packet": key[1], **detail}
            if matches:
                clips.append({
                    "candidate_id": candidate["candidate_id"],
                    "before_tags": candidate["tags"],
                    "trial_tags": sorted(set(candidate["tags"]) | {TAG}),
                    "before_score": candidate["overall_score"],
                    "trial_score": candidate["overall_score"],
                    "kills": list(matches.values()),
                })
        result["thresholds"][threshold] = {"recorded_qualifying_deaths": len(qualified), "clips": clips}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("--candidates", type=Path, help="Use an existing candidate stream from this export.")
    args = parser.parse_args()
    print(json.dumps(analyze(args.export, args.candidates or args.export / "frag_candidates.ndjson"), indent=2))


if __name__ == "__main__":
    main()
