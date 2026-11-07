"""Tests for tools/replay_coverage_audit.py (run: python3 -m unittest discover -s tools -p 'test_*.py')."""
import unittest

from replay_coverage_audit import parse


class ReplayCoverageAuditTests(unittest.TestCase):
    def test_replay_partitions_and_semantic_scope(self):
        log = """
## [CHUNKED] SOURCE FACT INVENTORY: 235 distinct statement/evidence pairs from 238 candidate facts; identities not merged
## [CHUNKED] STATEMENT LOCATORS: total=238 located=124 missing=114 exact_match=124 mismatch=0 different_segment=61 invalid_window=0 (diagnostic only)
## [CHUNKED] PARAMETER COVERAGE: unresolved=95 matched_parameter_definitions=78 unmatched_parameter_facts=13 non_parameter_facts=4 (exact source evidence; read-only)
## [CHUNKED] SEMANTIC EVIDENCE DISPOSITION: lexical_exact=9 relational_exact=1 structural_headings=1 reserved_boundaries=2 unresolved=0 total=13
## [CHUNKED] IDENTITY REPLAY (READ ONLY): projected=141 duplicate_occurrences=2 positioned=143 unresolved=95 messages=2
## [CHUNKED] IDENTITY REPLAY MESSAGE: messages/g.md facts=100
## [CHUNKED] IDENTITY REPLAY MESSAGE: messages/w.md facts=41
"""
        report = parse(log)
        self.assertEqual(report["checks"], {
            "locator_partition": "pass",
            "exact_matches_within_located": "pass",
            "parameter_partition": "pass",
            "semantic_partition": "pass",
            "semantic_scope": "pass",
            "replay_partition": "pass",
            "replay_message_count": "pass",
            "replay_message_sum": "pass",
            "candidate_partition": "pass",
        })
        self.assertEqual(report["metrics"]["locators"]["missing"], 114)

    def test_inconsistent_counts_are_reported(self):
        log = """
## [CHUNKED] PARAMETER COVERAGE: unresolved=5 matched_parameter_definitions=2 unmatched_parameter_facts=2 non_parameter_facts=0
## [CHUNKED] SEMANTIC EVIDENCE DISPOSITION: lexical_exact=2 relational_exact=0 structural_headings=0 reserved_boundaries=0 unresolved=0 total=2
"""
        report = parse(log)
        self.assertEqual(report["checks"]["parameter_partition"], "fail")
        self.assertEqual(report["checks"]["semantic_partition"], "pass")
        self.assertIn("replay", report["missing_sections"])

    def test_empty_log_does_not_claim_success(self):
        report = parse("unrelated output")
        self.assertEqual(len(report["missing_sections"]), 6)
        self.assertEqual(report["checks"], {})


if __name__ == "__main__":
    unittest.main()
