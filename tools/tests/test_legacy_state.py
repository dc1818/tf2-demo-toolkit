import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("legacy", Path(__file__).parents[1] / "compare_legacy_state.py")
legacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(legacy)


class EvidenceParityTests(unittest.TestCase):
    def compare(self, extra=None, health=10, second_delta=False, approved=True):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            player = {"entity_id": 7, "user_id": 78, "health": 10}
            baseline = {"packet_sequence": 1, "players": [player], "removed_players": [], "projectiles": [], "removed_projectiles": []}
            trial_player = dict(player, health=health, **{key: None for key in legacy.FIELDS},
                                damage_blocked_total=0, medigun_target={"entity_id": None})
            trial_player.update(extra or {})
            trial = dict(baseline, players=[trial_player])
            rows = [trial]
            if second_delta:
                rows.append(dict(trial, packet_sequence=2, players=[dict(trial_player, damage_blocked_total=43)]))
            (root / "old").write_text(json.dumps(baseline) + "\n")
            (root / "new").write_text("\n".join(map(json.dumps, rows)))
            return legacy.compare(root / "old", root / "new", approved)

    def test_evidence_only_delta_preserves_reconstructed_legacy_state(self):
        result = self.compare(second_delta=True)
        self.assertTrue(result["legacy_state_parity"])
        self.assertEqual(result["state_packets_compared"], 2)

    def test_optional_values_remain_distinct_and_typed(self):
        for extra in [{"damage_blocked_total": None}, {"medigun_target": None}, {"medigun_target": {"entity_id": 6}}]:
            self.assertTrue(self.compare(extra)["legacy_state_parity"])
        for extra in [{"damage_blocked_total": False}, {"damage_blocked_total": -1},
                      {"medigun_target": {}}, {"medigun_target": {"entity_id": False}}]:
            with self.assertRaises(ValueError):
                self.compare(extra)

    def test_real_state_changes_cannot_hide_behind_additive_fields(self):
        with self.assertRaisesRegex(ValueError, "legacy state differs"):
            self.compare(health=11)

    def test_default_comparison_does_not_ignore_unapproved_fields(self):
        with self.assertRaisesRegex(ValueError, "legacy state differs"):
            self.compare(approved=False)
