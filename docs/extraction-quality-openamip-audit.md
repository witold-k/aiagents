# OpenAMIP extraction quality audit — baseline

## Input and version boundary

- Source: OpenAMIP Standard v1.17 Revision F, converted Markdown.
- Compared artifact: user's unverified JSON export (facts.json, manifest.json, 26 message JSON files, general.json, infrastructure.json), **not** the newer read-only identity replay projection.
- The export contains 253 facts marked `supported` and 24 marked `unresolved`; the replay reports 30 message identities and 285 projected facts. These are different stages and must not be conflated.
- This audit does **not** assert semantic completeness.

## Three representative cases

### A — simple fixed-arity command (`messages/upper_a.json`)

Observed: a single `supported` statement, copied from a flattened source table row:
`Type: A | Alive intervalAntenna should send a status message at least this often. 0 means never repeat. | 1 | int interval, seconds`.

- **Positive:** identity, interval meaning, zero semantics, count, type and unit are present in source-backed text.
- **Defect:** none is modeled as a field; parameter cardinality and type cannot be checked without parsing the prose/table row again.
- **Needed:** separate message description, positional parameter with index/type/unit, and zero behavior; preserve exact source evidence.

### B — fixed positional plus optional indexed key/value (`messages/upper_b.json`)

Observed: `supported` text contains `2(minimum)`, Rx/Tx LO frequency descriptions, optional `rx(N)`/`tx(N)`, examples and additional device-indexing rules.

- **Positive:** much of the necessary material survives.
- **Defects:** parameter definitions are concatenated (`MHzfloat`); an example is flattened (`rx2=18200rx3=...`); partial and overlapping facts are separately marked `supported` (e.g. `at Tx LO frequency, MHz`).
- **Needed:** distinguish the two positional float parameters from indexed optional keys, independently preserve examples, and flag truncated/non-atomic statements.

### g — status response with conditional fields (`messages/g.json`)

Observed: the `supported` facts cover response to G, meanings of status bits, optional fifth/sixth/seventh fields, RF-chain constraints, and key/value extensions.

- **Positive:** substantial conditional information is captured.
- **Defects:** status-bit definitions are also copied into `infrastructure.md`, showing semantic duplication; several facts share coarse source offsets; a sentence about the fourth parameter loses its condition when isolated (`The fourth parameter 'Tx Disabled' should be set to 0.`), though the surrounding evidence includes conditions.
- **Needed:** explicit parameter positions, conditions, and cross-reference/ownership rather than duplication.

## Cross-cutting findings

1. `supported` means a quote was accepted; it does not imply atomicity, valid structured syntax, or sufficient context.
2. Flattened PDF-to-Markdown tables create concatenated parameter names and descriptions; source conversion quality and LLM extraction quality must be measured separately.
3. Some output facts lose condition clauses or are truncated; quote verification alone cannot detect this.
4. The exported manifest is `unverified`; do not present it as an implementation-ready protocol schema.
5. The newer replay does not automatically rewrite the old JSON export. A later end-to-end run or projection/export stage must be evaluated independently.

## Suggested generic acceptance tests

- Every message has a case-sensitive identity and source reference.
- Each parameter has distinct position or key pattern, data type, cardinality, optionality, and constraints when present in source.
- A statement is flagged if it is a partial fragment or joins multiple independent requirements.
- Conditions are represented with the requirement they qualify.
- Examples remain exact, not whitespace-joined.
- Overlapping evidence deduplicates without dropping independent constraints.
- Completeness is measured against a manually curated reference subset, including expected missing items, not only against LLM-generated facts.

## Next step

Build a small, manually verified fixture for A, B, and g, then compare the produced structured representation with it. Prefer generic schema and validation changes over OpenAMIP-specific heuristics.
