# Code Fix Design

Design one concrete minimal repair for the supplied diagnosis and constraints.

Use the diagnostics and source context to choose a mechanism that is valid under the language rules and still supports the failing use case.

Name the concrete language or library mechanism. Verify that the proposed member representation can actually exist in every required state.

You may make reasonable design decisions when the source does not determine a single repair.

Do not modify files or emit tool calls.

Return only the concise repair design for the patching agent.
