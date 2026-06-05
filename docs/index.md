# nutlog Documentation

`nutlog` is a simple, local-first, single-user CLI tool for logging food purchases, tracking nutrition data, recording consumption, and generating basic reports.

It is designed to be **LLM-agent friendly** (via consistent commands and `--json` output) while remaining usable directly by humans.

- **Project home**: see top-level [README.md](../README.md)
- **For agents and contributors**: [AGENTS.md](../AGENTS.md) and [CODING_PRACTICES.md](../CODING_PRACTICES.md)

## Documentation Index

- [Installation](installation.md) — How to install from source, AUR, or package.
- [Getting Started](getting-started.md) — First steps, database location, basic workflow.
- [Command Reference](command-reference.md) — Detailed reference for every `nutlog <entity> <action>`.
- [Nutrition Tracking](nutrition-tracking.md) — How nutritional information is modeled, set, and used.
- [Reporting](reporting.md) — Nutrition summaries and spending reports.
- [Agent & JSON Usage](agent-usage.md) — Using `nutlog` from LLM agents or scripts (the primary intended interface).
- [Date Inputs, Money, and Data Model](data-model.md) — Flexible dates, cents, DB schema notes.
- [Troubleshooting](troubleshooting.md) — Common issues and solutions.

## Quick Command Structure

```
nutlog [GLOBAL FLAGS] <ENTITY> <ACTION> [options/args]
```

Global flags (apply to all):

- `--json` : emit machine-readable JSON (recommended for agents/scripts)
- `--db /path/to/db.sqlite` : override default DB location
- `--quiet` : suppress non-essential human output
- `--help`, `--version`

Entities (see command-reference.md for full actions):

- `product`
- `nutrient`
- `product-tag`
- `purchase`
- `store`
- `store-tag`
- `consumption`
- `report`

All mutating actions return a `{ "success": true, "id": N, "message": "..." }` object under `--json`.

## Key Design Principles

- Everything is stored in a single local SQLite database (XDG data dir by default).
- Money is **always** handled and stored in integer cents internally.
- All timestamps are stored in UTC; human output converts to local time.
- Fuzzy search uses Jaro-Winkler similarity (simple, no external deps beyond `strsim`).
- Pre-populated common nutrients on first run.
- No network, no cloud, no accounts. Local and private.
- Safety: deleting a product with purchases requires `--force` (cascades purchases).

## Version and Help

```bash
nutlog --version
nutlog --help
nutlog product --help
nutlog product create --help
```

Help text in the CLI is the authoritative source for flag names and syntax.

## License

See top-level README (currently TBD / personal tool).

---

*This documentation is installed alongside the binary for offline reference (typically under `/usr/share/doc/nutlog/docs/`).*
