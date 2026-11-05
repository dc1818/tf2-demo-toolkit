#!/usr/bin/env python3
"""Read-only deployment context. Does not infer damage prevented or a life saved."""

import argparse
from collections import deque
from heapq import merge
from itertools import groupby
import json
import math
from pathlib import Path
import sys


def uid(value):
    return value if type(value) is int and 0 < value < 65535 else None


def records(path, phase):
    previous = -1
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                row = json.loads(line)
                if not isinstance(row, dict):
                    raise ValueError("expected an object")
                packet = row.get("packet_sequence")
                if type(packet) is not int or packet < previous or packet < 0:
                    raise ValueError("expected ordered packet_sequence")
                if phase == 0 and not isinstance(row.get("event"), dict):
                    raise ValueError("expected event object")
                if phase == 1 and any(not isinstance(row.get(key), list)
                                      for key in ("players", "removed_players")):
                    raise ValueError("expected player arrays")
            except ValueError as error:
                raise ValueError(f"{path}:{line_number}: {error}") from error
            previous = packet
            yield (packet, phase, line_number), row


def server_tick(row):
    tick = row.get("server_tick")
    return tick if type(tick) is int and tick > 0 else None


def probe(directory, rate, seconds=8):
    states, entities = {}, {}
    recent_damage = deque()
    pending = []
    max_age = rate * .5

    def boundary(user, reason):
        states.pop(user, None)
        for result in pending:
            if user in (result["recipient"], result["medic"]):
                result["boundary"] = reason

    def pre_state(user, tick):
        sample = states.get(user)
        if not sample or tick is None or not 0 <= tick - sample[0] <= max_age:
            return None
        player = sample[1]
        if player.get("in_pvs") is not True or player.get("life_state") != "alive":
            return None
        return player

    def close(result, tick, reason):
        result["window_end_reason"] = reason
        result["window_end_server_tick"] = tick
        result["result"] = "context_only"
        result["life_saved"] = "unknown"
        result["damage_prevented"] = None
        result.pop("_identity", None)
        return result

    stream = merge(
        records(directory / "events.ndjson", 0),
        records(directory / "state_samples.ndjson", 1),
        key=lambda item: item[0],
    )
    last_tick = None
    for _, batch in groupby(stream, key=lambda item: item[0][0]):
        packet = list(batch)
        ticks = [server_tick(row) for _, row in packet if server_tick(row)]
        tick = ticks[0] if ticks else None
        if tick and last_tick and tick < last_tick:
            for result in pending:
                yield close(result, last_tick, "clock_regression")
            pending.clear()
            states.clear()
            entities.clear()
            recent_damage.clear()
        if tick:
            last_tick = tick
            while recent_damage and tick - recent_damage[0][0] > rate:
                recent_damage.popleft()
            for result in pending[:]:
                if tick - result["server_tick"] > seconds * rate:
                    pending.remove(result)
                    yield close(result, result["server_tick"] + seconds * rate, "time_limit")

        for (_, phase, _), row in packet:
            if phase:
                continue
            kind, data = row.get("event_type"), row["event"]
            event_tick = server_tick(row)
            user = uid(data.get("user_id", data.get("userid")))
            if kind in {"teamplay_round_start", "teamplay_round_active", "teamplay_round_win",
                        "teamplay_round_stalemate", "round_start"}:
                for result in pending:
                    yield close(result, event_tick, "round_boundary")
                pending.clear()
                states.clear()
                entities.clear()
                recent_damage.clear()
            if kind == "player_chargedeployed":
                medic, target = user, uid(data.get("target_id"))
                # A second deployment bounds the first medic's context.
                for result in pending[:]:
                    if result["medic"] == medic and medic:
                        pending.remove(result)
                        yield close(result, event_tick, "next_deployment")
                before = pre_state(target, event_tick)
                medic_state = pre_state(medic, event_tick)
                team = before.get("team") if before else None
                valid_damage = [
                    {"server_tick": t, "damage": amount, "attacker": attacker}
                    for t, victim, amount, attacker in recent_damage if victim == target
                    and event_tick is not None and 0 <= event_tick - t <= rate
                ]
                result = {
                    "detector": "uber_context_v1", "packet_sequence": row["packet_sequence"],
                    "demo_tick": row.get("demo_tick"), "server_tick": event_tick,
                    "medic": medic, "recipient": target,
                    "medigun_reported": medic_state.get("medigun") if medic_state else None,
                    "recipient_pre_health": before.get("health") if before else None,
                    "recent_damage": valid_damage, "invulnerability_events": [],
                    "recipient_kills": [], "same_team_kills": [],
                    "recipient_death_ticks": [], "last_observed_alive_tick": None,
                    "boundary": None,
                    "_identity": (before.get("entity_id"), before.get("steam_id"), team)
                        if before else None,
                }
                if event_tick is None:
                    yield close(result, None, "missing_server_clock")
                else:
                    pending.append(result)
            if kind == "player_hurt":
                amount = data.get("damage_amount")
                if event_tick and user and type(amount) is int and amount > 0:
                    recent_damage.append((event_tick, user, amount, uid(data.get("attacker"))))
            if kind == "player_death":
                attacker = uid(data.get("attacker"))
                a = pre_state(attacker, event_tick)
                v = pre_state(user, event_tick)
                for result in pending:
                    if user == result["recipient"] and event_tick:
                        result["recipient_death_ticks"].append(event_tick)
                    if result["boundary"] or not event_tick or not attacker or attacker == user:
                        continue
                    if attacker == result["recipient"]:
                        result["recipient_kills"].append({
                            "server_tick": event_tick, "demo_tick": row.get("demo_tick"),
                            "victim": user,
                        })
                    identity = result["_identity"]
                    if (identity and identity[2] in ("Red", "Blue", "red", "blu", "blue")
                            and a and v and a.get("team") == identity[2]
                            and v.get("team") != identity[2]
                            and v.get("team") in ("Red", "Blue", "red", "blu", "blue")):
                        result["same_team_kills"].append(event_tick)
            if kind in {"player_spawn", "player_disconnect", "player_team", "player_changeclass",
                        "player_death"} and user:
                boundary(user, kind)

        # Activation/invulnerability event order within a packet is not
        # guaranteed. Match only the exact initial target and named medic.
        for (_, phase, _), row in packet:
            if phase or row.get("event_type") != "player_invulned":
                continue
            data, t = row["event"], server_tick(row)
            for result in pending:
                if (t is not None and not result["boundary"]
                        and result["medic"] == uid(data.get("medic_user_id"))
                        and result["recipient"] == uid(data.get("user_id"))):
                    result["invulnerability_events"].append(t)

        # Samples represent state after all events in this packet.
        for (_, phase, _), row in packet:
            if not phase:
                continue
            for entity in row["removed_players"]:
                user = entities.pop(entity, None)
                if user:
                    boundary(user, "entity_removed")
            for player in row["players"]:
                if not isinstance(player, dict):
                    raise ValueError("expected a player object")
                entity, user = player.get("entity_id"), uid(player.get("user_id"))
                old_user = entities.pop(entity, None)
                if old_user and old_user != user:
                    boundary(old_user, "entity_reassigned")
                if not user:
                    continue
                entities[entity] = user
                t = server_tick(row)
                identity = (entity, player.get("steam_id"), player.get("team"))
                for result in pending:
                    if user != result["recipient"] or result["boundary"]:
                        continue
                    if result["_identity"] is not None and identity != result["_identity"]:
                        result["boundary"] = "identity_changed"
                    elif (t and player.get("in_pvs") is True
                          and player.get("life_state") == "alive"):
                        result["last_observed_alive_tick"] = t
                if t:
                    states[user] = t, player

    for result in pending:
        yield close(result, last_tick, "end_of_export")


def positive(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("expected a finite positive number")
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("--tick-rate", required=True, type=positive)
    args = parser.parse_args()
    try:
        for result in probe(args.export, args.tick_rate):
            print(json.dumps(result))
    except (OSError, ValueError, TypeError) as error:
        print(error, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
