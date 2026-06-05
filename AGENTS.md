# agents.md — Guidelines for LLM Agents Working on nutlog

This document helps AI agents (and humans working with them) collaborate effectively on the `nutlog` project.

## Project Philosophy

`nutlog` is a **single-user, local-first, LLM-agent-first** CLI tool for logging food purchases and nutrition.

Key principles:
- **Simplicity first** — Prefer boring, maintainable solutions over clever abstractions.
- **Agent-friendly by design** — The CLI itself is built to be easily used by LLM agents via tools/skills.
- **Predictability > magic** — Consistent command structure, clear output, and explicit behavior.

## How to Work as an Agent on This Project

### 1. Exploration
- Start by reading `CODING_PRACTICES.md`, `agents.md`, and the main `README.md`.
- Use `cargo tree`, `cargo metadata --format-version 1`, and `rg` / `grep` for code exploration.
- Prefer reading source files directly rather than relying only on summaries.
- When in doubt about architecture, look at existing commands for patterns.

### 2. Making Changes
- Follow the command pattern: `nutlog <entity> <action> [flags]`
  - Entities: `product`, `purchase`, `consumption`, `store`, `product-tag`, `store-tag`, `nutrient`, etc.
- Always support `--json` output for commands that return data.
- Use `clap` derive macros for all new commands.
- Add proper help text — this is the primary documentation for agents.
- Update `agents.md` and `CODING_PRACTICES.md` when you introduce new patterns.

### 3. Code Quality
- Run `cargo fmt` and `cargo clippy -- -D warnings` before proposing changes.
- Use the configurations in `rustfmt.toml` and `clippy.toml`.
- Prefer `thiserror` for domain errors and `anyhow` only at the binary boundary.
- Avoid `unwrap()`, `expect()`, and `panic!()` in normal execution paths.

### 4. CLI Design Rules (Critical for Agent Usability)
When adding or modifying commands:
- Keep subcommands consistent (`create`, `list`, `show`, `search`, `edit`, `delete`).
- Use `--json` as the standard way for agents to get structured data.
- Human-readable output should be clean and tabular when appropriate.
- Date inputs should accept flexible formats (`today`, `yesterday`, `2026-06-04`, etc.).
- Money should always be handled in cents internally.
- Error messages must be actionable and not assume human context only.

### 5. Database & Schema Changes
- Use migrations (sqlx or similar) for all schema changes.
- Never write raw SQL strings in business logic when possible.
- Keep the data model simple — this is a personal tool, not an enterprise system.
- Timestamps are stored in UTC.

### 6. Testing
- Add tests for new functionality (especially command parsing and JSON output).
- Use `assert_cmd` + `predicates` for CLI integration tests.
- Keep tests fast and focused.

### 7. Documentation
- Update command help text as the source of truth.
- Keep `CODING_PRACTICES.md` and this file up to date.
- When adding significant features, consider adding an example in the README.

## Common Tasks for Agents

| Task                        | Recommended Approach                              |
|----------------------------|---------------------------------------------------|
| Add new command            | Follow existing `clap` derive pattern + `--json`  |
| Modify nutritional model   | Update JSON structures + migration if needed      |
| Improve fuzzy search       | Keep it simple; document the algorithm            |
| Add reporting feature      | Create `nutlog report ...` subcommand             |
| Change output format       | Support both human and `--json` modes             |
| Refactor for agents        | Make commands more predictable and self-documenting |

## Things Agents Should Avoid

- Introducing web frameworks, async runtimes, or complex state unless clearly justified.
- Breaking the `nutlog <entity> <action>` command structure.
- Removing `--json` support from existing commands.
- Using unclear abbreviations in command/flag names.
- Adding features that make the tool harder for other agents to use.

## Quick Reference

```bash
# Format & lint
cargo fmt
cargo clippy -- -D warnings

# Run tests
cargo test

# Typical development flow
cargo run -- product create "..." --json
cargo run -- --json product list
```

---

**Goal**: Make `nutlog` a joy for both humans *and* LLM agents to use and extend.

Update this file whenever the project's agent-interaction patterns evolve.
