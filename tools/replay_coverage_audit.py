#!/usr/bin/env python3
"""Read-only audit of [CHUNKED] identity replay diagnostics.

Uses only the Python standard library. Does not infer semantic correctness,
message ownership, or source coverage from diagnostic counts.
"""
import argparse
import json
import re
import sys
from pathlib import Path

PATTERNS = {
    "inventory": r"SOURCE FACT INVENTORY: (?P<distinct>\d+) distinct statement/evidence pairs from (?P<candidates>\d+) candidate facts",
    "identity": r"IDENTITY CLASSIFICATION: (?P<typed>\d+) explicitly typed candidates, (?P<unresolved_candidates>\d+) unresolved candidates \((?P<unresolved_candidate_facts>\d+) facts\)",
    "locators": r"STATEMENT LOCATORS: total=(?P<total>\d+) located=(?P<located>\d+) missing=(?P<missing>\d+) exact_match=(?P<exact>\d+) mismatch=(?P<mismatch>\d+) different_segment=(?P<different_segment>\d+) invalid_window=(?P<invalid_window>\d+)",
    "parameters": r"PARAMETER COVERAGE: unresolved=(?P<unresolved>\d+) matched_parameter_definitions=(?P<matched>\d+) unmatched_parameter_facts=(?P<unmatched>\d+) non_parameter_facts=(?P<other>\d+)",
    "semantic": r"SEMANTIC EVIDENCE DISPOSITION: lexical_exact=(?P<lexical>\d+) relational_exact=(?P<relational>\d+) structural_headings=(?P<headings>\d+) reserved_boundaries=(?P<reserved>\d+) unresolved=(?P<unresolved>\d+) total=(?P<total>\d+)",
    "replay": r"IDENTITY REPLAY \(READ ONLY\): projected=(?P<projected>\d+) duplicate_occurrences=(?P<duplicates>\d+) positioned=(?P<positioned>\d+) unresolved=(?P<unresolved>\d+) messages=(?P<messages>\d+)",
}
MESSAGE = re.compile(r"^## \[CHUNKED\] IDENTITY REPLAY MESSAGE: (?P<path>messages/[^ ]+\.md) facts=(?P<count>\d+)$", re.M)


def parse(text):
    sections = {}
    for key, pattern in PATTERNS.items():
        matches = list(re.finditer(pattern, text))
        if matches:
            sections[key] = {name: int(value) for name, value in matches[-1].groupdict().items()}
    messages = {m.group("path"): int(m.group("count")) for m in MESSAGE.finditer(text)}
    checks = {}
    def check(name, condition):
        checks[name] = "pass" if condition else "fail"

    if "locators" in sections:
        x = sections["locators"]
        check("locator_partition", x["located"] + x["missing"] == x["total"])
        check("exact_matches_within_located", x["exact"] <= x["located"])
    if "parameters" in sections:
        x = sections["parameters"]
        check("parameter_partition", x["matched"] + x["unmatched"] + x["other"] == x["unresolved"])
    if "semantic" in sections and "parameters" in sections:
        x = sections["semantic"]
        check("semantic_partition", sum(x[k] for k in ("lexical", "relational", "headings", "reserved", "unresolved")) == x["total"])
        check("semantic_scope", x["total"] == sections["parameters"]["unmatched"])
    if "replay" in sections:
        x = sections["replay"]
        check("replay_partition", x["projected"] + x["duplicates"] == x["positioned"])
        check("replay_message_count", len(messages) == x["messages"])
        check("replay_message_sum", sum(messages.values()) == x["projected"])
        if "inventory" in sections:
            check("candidate_partition", x["positioned"] + x["unresolved"] == sections["inventory"]["candidates"])
    return {
        "schema_version": 1,
        "metrics": sections,
        "messages": dict(sorted(messages.items())),
        "checks": checks,
        "missing_sections": sorted(set(PATTERNS) - set(sections)),
        "interpretation": {
            "locator_missing": "No statement locator; not proof that source evidence is absent",
            "semantic_disposition": "Source structure and quote location only; not semantic verification",
            "replay": "Read-only projection; does not prove that exported JSON matches",
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path, help="captured replay stdout")
    parser.add_argument("--output", type=Path, help="optional output JSON path")
    args = parser.parse_args()
    report = parse(args.log.read_text(encoding="utf-8", errors="replace"))
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    if report["missing_sections"] or "fail" in report["checks"].values():
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
