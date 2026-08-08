🧩 Core idea

=> causality graph

Every Rust error belongs to a dependency tier.
You always fix the earliest tier that has at least one error.

This prevents your agent from touching downstream errors that will disappear automatically.
🧱 The 7‑tier ordering model (recommended)

Each bullet begins with a Guided Link so you can explore each tier deeper.
1. Syntax errors

These break parsing and make all other diagnostics unreliable.

Examples:

    unexpected token

    mismatched delimiters

    unclosed delimiter

    missing ;

    stray }

Rule: Fix all syntax errors before touching anything else.
2. Missing items

These include missing imports, missing types, missing modules.

Examples:

    E0433: failed to resolve

    E0412: cannot find type

    E0425: cannot find value

Why second?
Missing items cause dozens of fake type errors.
3. Type mismatch

These are the classic E0308, E0282, E0283, inference failures.

Examples:

    expected X, found Y

    cannot infer type

    mismatched types

Why third?
Once syntax and missing items are fixed, type errors become meaningful.
4. Trait/method resolution

Examples:

    E0599: no method named

    E0277: trait bound not satisfied

Why fourth?
Trait resolution depends on correct types and imports.
5. Borrow checker

Examples:

    E0499: cannot borrow as mutable more than once

    E0502: cannot borrow because it is already borrowed

    E0507: cannot move out of borrowed content

Why fifth?
Borrow errors often vanish after fixing types or traits.
6. Lifetime errors

Examples:

    E0621: lifetime mismatch

    E0623: lifetime may not live long enough

Why sixth?
Lifetimes depend on correct ownership and borrow structure.
7. Lints & warnings

These are non‑blocking and should be handled last.
