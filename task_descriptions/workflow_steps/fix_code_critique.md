# Code Fix Critique

Validate the candidate fix analysis against the supplied source, diagnostics, and language rules.

Do not modify files and do not emit tool calls.

Check exactly these two things:

1. MECHANISM
Does the concrete proposed code mechanism actually have the effect the candidate claims under the language rules?

Check construction, initialization, ownership, lifetime, overload resolution, and type rules. Account for operations performed implicitly even when omitted from the proposed source change.

For object members, distinguish omitting an explicit initializer from omitting construction. If the language still implicitly initializes or constructs a declared member, evaluate that implicit operation and its requirements before accepting a claim that removing the explicit initializer avoids construction.

Reject if the mechanism cannot work as claimed.

2. POSTCONDITION
After the proposed repair, is the concrete operation required by the original failing use case still available and valid?

Reject if the repair removes, disables, constrains away, or otherwise makes the required operation unavailable.

Reject only when there is concrete evidence that the proposed repair is wrong. Concrete evidence must come from the supplied source, diagnostics, or language rules.

Do not reject because another intention, behavior, implementation, or design could hypothetically exist unless the supplied context provides evidence for it. Absence of proof that the candidate is correct is not by itself a reason to reject.

If the proposed mechanism is valid and no concrete defect in the repair has been identified, accept it. Do not reject defects in the explanation, justification, or analysis that do not make the proposed repair itself invalid.

Do not infer design intent.
Do not evaluate whether another design would be better.
Do not propose or compare alternative repairs.

Return exactly one of these forms:

ACCEPT

or:

REJECT
CHECK: MECHANISM or POSTCONDITION
EVIDENCE: <concrete source fact, diagnostic, or language rule>
DEFECT: <why the candidate fails this check>
