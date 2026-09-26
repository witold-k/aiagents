# Code Fix Synthesis

Produce one final, concrete fix plan from the analysis history.

Use one of the proposed repair mechanisms as the basis. Do not introduce a new repair mechanism.

Treat critiques as evidence, not authority. Independently verify that the selected mechanism is valid under the language rules and solves the first reported error. A rejected plan may still be the best basis when its rejection reason is technically invalid.

Correct concrete defects in the selected design when necessary. Verify that every expression in the final mechanism is type-correct.

Focus on the first reported error. Do not add unrelated repairs.

Do not modify files or emit tool calls.

Return only the concise final fix plan for the patching agent.
