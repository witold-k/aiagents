# Code Fix Synthesis

Produce one final, concrete fix plan from the analysis history.

Treat critiques as evidence, not authority. Independently verify that the final mechanism is valid under the language rules and solves the first reported error. A rejected plan may still be the best basis when its rejection reason is technically invalid.

Discard mechanisms only when their defect is valid. Preserve useful findings from rejected plans when they remain technically sound.

Focus on the first reported error. Do not add unrelated repairs.

Do not modify files or emit tool calls.

Return only the concise final fix plan for the patching agent.
