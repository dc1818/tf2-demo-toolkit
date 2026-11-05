import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "probe_uber_context.py"
spec = importlib.util.spec_from_file_location("uber_context", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def event(packet, tick, kind, **payload):
    return {"packet_sequence": packet, "server_tick": tick, "demo_tick": packet,
            "event_type": kind, "event": payload}


def player(user=2, health=30, **extra):
    return {"entity_id": user + 10, "user_id": user, "health": health,
            "steam_id": str(user), "team": "Red", "life_state": "alive",
            "in_pvs": True, **extra}


def state(packet, tick, *players):
    return {"packet_sequence": packet, "server_tick": tick,
            "players": list(players), "removed_players": []}


class UberContextTests(unittest.TestCase):
    def run_probe(self, events, states):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, rows in (("events", events), ("state_samples", states)):
                (root / f"{name}.ndjson").write_text(
                    "".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            before = {p.name: p.read_bytes() for p in root.iterdir()}
            result = list(module.probe(root, 100))
            self.assertEqual(before, {p.name: p.read_bytes() for p in root.iterdir()})
            return result

    def deploy(self, packet=2, tick=101):
        return event(packet, tick, "player_chargedeployed", user_id=1, target_id=2)

    def base(self):
        return [state(1, 100, player(), player(1, 150, medigun="uber"))]

    def test_context_records_damage_and_outcome_but_never_claims_save(self):
        events = [event(1, 100, "player_hurt", user_id=2, attacker=3, damage_amount=90),
                  self.deploy(), event(2, 101, "player_invulned", user_id=2, medic_user_id=1),
                  event(4, 120, "player_death", user_id=3, attacker=2)]
        states = self.base() + [state(3, 110, player(), player(3, team="Blue"))]
        row, = self.run_probe(events, states)
        self.assertEqual(row["recent_damage"][0]["damage"], 90)
        self.assertEqual(row["recipient_pre_health"], 30)
        self.assertEqual(len(row["recipient_kills"]), 1)
        self.assertEqual(len(row["same_team_kills"]), 1)
        self.assertEqual(row["life_saved"], "unknown")
        self.assertIsNone(row["damage_prevented"])

    def test_invuln_event_before_deploy_still_matches_same_packet(self):
        row, = self.run_probe(
            [event(2, 101, "player_invulned", user_id=2, medic_user_id=1), self.deploy()],
            self.base())
        self.assertEqual(row["invulnerability_events"], [101])

    def test_wrong_medic_or_recipient_cannot_prove_protection(self):
        row, = self.run_probe(
            [self.deploy(), event(3, 102, "player_invulned", user_id=2, medic_user_id=4)],
            self.base())
        self.assertEqual(row["invulnerability_events"], [])

    def test_kritz_is_reported_without_assuming_invulnerability(self):
        row, = self.run_probe([self.deploy()],
            [state(1, 100, player(), player(1, medigun="kritzkrieg"))])
        self.assertEqual(row["medigun_reported"], "kritzkrieg")
        self.assertEqual(row["invulnerability_events"], [])
        self.assertEqual(row["life_saved"], "unknown")

    def test_no_death_event_is_not_assumed_survival(self):
        row, = self.run_probe([self.deploy()], self.base())
        self.assertIsNone(row["last_observed_alive_tick"])
        self.assertEqual(row["window_end_reason"], "end_of_export")

    def test_death_and_respawn_stop_recipient_kill_attribution(self):
        row, = self.run_probe(
            [self.deploy(), event(3, 110, "player_death", user_id=2, attacker=3),
             event(4, 115, "player_spawn", user_id=2),
             event(6, 130, "player_death", user_id=3, attacker=2)],
            self.base() + [state(5, 120, player())])
        self.assertEqual(row["recipient_death_ticks"], [110])
        self.assertEqual(row["recipient_kills"], [])
        self.assertIsNone(row["last_observed_alive_tick"])

    def test_round_boundary_stops_unrelated_later_win_context(self):
        row, = self.run_probe(
            [self.deploy(), event(3, 120, "teamplay_round_win", team=2),
             event(5, 130, "player_death", user_id=3, attacker=2)], self.base())
        self.assertEqual(row["window_end_reason"], "round_boundary")
        self.assertEqual(row["recipient_kills"], [])

    def test_stale_or_same_packet_state_is_not_pre_deploy_health(self):
        for states in ([state(1, 10, player())], [state(2, 101, player())]):
            with self.subTest(states=states):
                row, = self.run_probe([self.deploy()], states)
                self.assertIsNone(row["recipient_pre_health"])

    def test_entity_reuse_invalidates_target(self):
        row, = self.run_probe([self.deploy()], self.base() +
            [state(3, 110, player(4, entity_id=12))])
        self.assertEqual(row["boundary"], "entity_reassigned")

    def test_clock_regression_closes_context(self):
        row, = self.run_probe([self.deploy(), event(3, 80, "player_healed")], self.base())
        self.assertEqual(row["window_end_reason"], "clock_regression")

    def test_time_limit_and_second_deployment_bound_context(self):
        rows = self.run_probe(
            [self.deploy(), self.deploy(3, 103), event(4, 1000, "player_healed")], self.base())
        self.assertEqual([r["window_end_reason"] for r in rows],
                         ["next_deployment", "time_limit"])

    def test_cli_rejects_invalid_rate(self):
        result = subprocess.run([sys.executable, str(SCRIPT), ".", "--tick-rate", "nan"],
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)

    def test_malformed_export_rejected(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "events.ndjson").write_text('{"packet_sequence":1,"event":[]}\n')
            (root / "state_samples.ndjson").write_text("")
            with self.assertRaises(ValueError):
                list(module.probe(root, 100))


if __name__ == "__main__":
    unittest.main()
