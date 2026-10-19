# Chunked Transform — Evidence-based Audit Decision

Compare the existing destination specification with the supplied bounded
evidence. Determine whether it contains an actionable omission, unsupported
claim, contradiction, or an unused/missing generic helper contract.

Return exactly one line:
UNCHANGED
or
REVISE

If this source window contains no relevant protocol definitions or other
facts that warrant a correction, return UNCHANGED, not the extraction-stage
marker NO_MESSAGE_DEFINITIONS. The runtime also accepts that marker as
UNCHANGED for compatibility with model output.

Choose REVISE only when the evidence warrants a concrete correction.
Do not propose generic infrastructure solely as a precaution. Do not
generate code or explanations.

Only revise for a specific fact explicitly present in the evidence. A partial
encoded payload, image, isolated identifier, or source fragment without a
complete factual assertion is not sufficient. Do not infer new protocol rules
from a single-character or truncated source excerpt. Preserve established
claims that this evidence cannot confirm or contradict. If the evidence
cannot justify a concrete correction, leave the specification unchanged.
