# Troubleshooting

## Common Issues

### "command not found: nutlog"

- After `cargo install --path .` make sure `~/.cargo/bin` is on your `$PATH`.
- After package install: check that `/usr/bin` is on PATH (it almost always is).
- Run `which nutlog` or `command -v nutlog`.

### Database not created / "failed to open database"

- The tool creates the directory on first use.
- Check permissions on `$XDG_DATA_HOME` or `~/.local/share`.
- If using `--db /some/path`, the parent must be writable by the user.

### "unrecognized date format"

The parser is intentionally limited. Use one of:

- `today`, `yesterday`
- `2026-06-05` (ISO)
- `3 days ago`
- `last week`

Run with a known-good date to test.

If you are an agent running on a different host than the "user's today", prefer explicit calendar dates.

### Product delete fails with "has associated purchases"

This is by design (data integrity).

```bash
nutlog product delete 5 --force   # also deletes the purchases
```

Only use `--force` when you truly want to remove history.

### Nutrient delete fails with "referenced by product nutrition data"

This is by design (data integrity).

```bash
nutlog nutrient delete 17 --force   # also removes product micronutrient rows
```

Only use `--force` when you accept losing those micronutrient entries on affected products.

### Purchase or consumption delete returns "not found"

- Confirm the ID with `nutlog --json purchase list` or `consumption list`.
- IDs are not reused after deletion; a stale ID from a previous session will fail.

### Nutrition report shows 0 or missing values

Possible causes:

- The consumed products do not have a `product nutrition set` row.
- Quantity or reference quantity was 0.
- Date filter excluded the consumption records.
- Unit mismatch (g vs ml) — no conversion is performed.

Use `nutlog --json product show <id>` to verify the `nutritional_information` block exists.

### Micronutrients never appear in reports

- You must have rows in `product_micronutrients` (normally created via `product nutrition set --micronutrient ...` or `--json-file`).
- The base `product_nutritions` row must also exist (the report query joins on it).
- Insert them directly or via future extended CLI.

### Prices look wrong (off by factor of 100)

You are probably looking at `price_cents` and treating it as dollars.

Always look at the formatted `price` / `amount` fields for human values, or divide cents by 100 yourself.

### Fuzzy search returns unexpected or no results

- Search is case-insensitive internally.
- Very short queries or very dissimilar names score low.
- Tag filter (`--tag`) is exact match, not fuzzy.
- Try `product list --json` and client-side filter if the built-in ranking doesn't suit your use case.

### "database error: ..."

Usually a constraint violation (unique name on nutrient, foreign key, etc.).

The error string from SQLite is passed through. Re-run the logical operation that should have prevented it.

### JSON parse errors in my agent

- The tool prints pretty JSON with 2-space indent via `serde_json::to_string_pretty`.
- On error paths it still prints to **stdout** the error envelope (not stderr).
- Always read stdout for the JSON payload; only use stderr for human diagnostics.
- Exit code is authoritative for success vs failure even when JSON is present.

### Timezone weirdness in dates

All storage is UTC. Display uses the local zone of the *process at display time*.

If you move the DB file between machines in different zones, the "local" fields will reflect the new machine's zone. The underlying instant is stable.

### Large histories / performance

The current implementation is a personal tool. It does full table scans for some reports and fuzzy searches load candidate lists into memory.

For a single user with a few years of data this is instantaneous. If you ever have tens of thousands of rows, consider adding more indexes or moving heavy reporting to SQL views (future work).

## Debugging / Inspection

```bash
# Use an explicit test DB
DB=/tmp/debug.db
nutlog --db $DB --json product list

# Open the SQLite directly
sqlite3 $DB
.tables
.schema products
SELECT * FROM purchases ORDER BY id DESC LIMIT 5;
```

Useful queries:

- Find products without any nutrition: left join and `WHERE reference_quantity IS NULL`
- List all consumption with scaled values (you can copy the logic from the report handler)

## Rebuilding / Reinstall after Source Change

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo install --path .
```

Or for packaged:

```bash
makepkg -fsi
```

## Reporting Bugs / Requesting Features

This is primarily a personal tool. Issues and PRs are welcome on the GitHub repository if they follow the spirit in [AGENTS.md](../AGENTS.md) and [CODING_PRACTICES.md](../CODING_PRACTICES.md).

When filing a bug, include:

- `nutlog --version`
- Exact command that failed
- The JSON or human output + exit code
- Whether you used `--db` or the default path
- OS / distro

## "It works on my machine"

Common environmental differences:

- Locale / timezone settings affecting date parsing and display
- `$XDG_DATA_HOME` being set to something unusual
- Different Rust / glibc versions (rarely an issue because of bundled SQLite)

## Still Stuck?

1. Read the command's `--help` again.
2. Read the relevant section in [command-reference.md](command-reference.md).
3. Try the operation with `--json` and a fresh `--db /tmp/test.db` to isolate.
4. Inspect the raw DB with `sqlite3`.
5. Look at the source of the handler (the code is intentionally kept straightforward).

The design goal is that an agent (or a human reading the docs + help) should be able to predict exactly what will happen.
