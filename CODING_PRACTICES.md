# Coding Practices for nutlog

This document defines the coding standards and best practices for the `nutlog` project.

## Goals

- Make the codebase pleasant for both humans and LLM agents to read and modify.
- Keep the CLI predictable, well-documented, and easy to script against.
- Follow idiomatic Rust while staying pragmatic for a single-user CLI tool.

## Core Principles

1. **LLM-Agent Friendly First**
   - Every command must support `--json` output when it makes sense.
   - Error messages should be clear, actionable, and structured.
   - Use consistent `entity action` subcommand pattern.
   - Prefer explicit over clever.

2. **Simplicity over Cleverness**
   - Single-user, local SQLite tool → avoid unnecessary abstractions.
   - Prefer `thiserror` + `anyhow` for errors.
   - Use `clap` derive API for commands (keeps everything in one place).

3. **Formatting & Style**
   - Run `cargo fmt` before committing (uses the project's `rustfmt.toml`).
   - Max line width: 100 characters.
   - Use 4-space indentation.

4. **Linting**
   - Run `cargo clippy -- -D warnings` in CI / before PRs.
   - See `clippy.toml` for project-specific rules.
   - Avoid `unwrap()`, `expect()`, `panic!()` in production paths. Use proper error handling.

5. **Error Handling**
   - Use `thiserror` for domain errors.
   - Use `anyhow` only at the binary entry point for nice error reporting.
   - Never silently ignore errors.

6. **Database & Data**
   - All timestamps stored in UTC.
   - Money always stored as integer cents.
   - Prefer explicit foreign keys and simple joins over complex ORMs.
   - Use `sqlx` (or `rusqlite` + migrations) with compile-time checked queries when possible.

7. **Testing**
   - Unit tests for pure logic.
   - Integration tests for CLI commands (using `assert_cmd` + `predicates`).
   - Keep tests fast — this is a local CLI, not a web service.

8. **Documentation**
   - Public items must have doc comments.
   - CLI help text is the primary user documentation.
   - Detailed user-facing documentation goes in the `docs/` directory (markdown). The `PKGBUILD` must be updated so the files are installed to `/usr/share/doc/$pkgname/docs/`.
   - Update `CODING_PRACTICES.md` when patterns change.

## Recommended Tooling

| Tool       | Command                        | When to run          |
|------------|--------------------------------|----------------------|
| rustfmt    | `cargo fmt`                    | Before every commit  |
| Clippy     | `cargo clippy -- -D warnings`  | Before every commit  |
| Tests      | `cargo test`                   | Before every commit  |
| Audit      | `cargo audit`                  | Periodically         |

## Useful Commands for Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo install cargo-edit cargo-audit
```

## References

- Official Rust Style Guide: https://doc.rust-lang.org/style-guide/
- rustfmt configuration: https://rust-lang.github.io/rustfmt/
- Clippy documentation: https://rust-lang.github.io/rust-clippy/
- Rust API Guidelines (for future library extraction): https://rust-lang.github.io/api-guidelines/

---

*This file is part of the nutlog coding practices bundle.*
