# Date Inputs, Money, Database & Internal Data Model

This document explains the "boring but important" details that both humans and agents need to know to use the tool correctly.

## Flexible Date Input

Used by:

- `purchase create --date`
- `consumption create --date`
- `report ... --since` / `--until`

### Supported Formats (case-insensitive after trimming)

- `today`
- `yesterday`
- `tomorrow`
- `YYYY-MM-DD` (e.g. `2026-06-05`) — recommended for precision
- `last week` (treated as -7 days)
- `last month` (treated as -30 days)
- `N days ago` or `N day ago` (e.g. `3 days ago`)
- Also accepts some US/UK variants like `MM-DD-YYYY`, `DD-MM-YYYY`
- Full RFC3339 timestamps (passed through)

### How "today" etc. Are Interpreted

1. The string is parsed relative to the **local calendar date** of the machine running `nutlog` (using `chrono::Local`).
2. The resulting naive date is turned into a local midnight `DateTime<Local>`.
3. That instant is converted to UTC and stored in the `*_at` columns as an RFC3339 string with `Z`.

Example (machine in UTC+2 on 2026-06-05 local):

- `--date today` → stored as `2026-06-04T22:00:00Z` (the UTC instant for local midnight).

### Human Display

When printing dates, `format_local` converts the stored UTC value back to the *current* local timezone of the viewer and shows `YYYY-MM-DD HH:MM:SS ZONE`.

JSON always includes both:

```json
{
  "utc": "2026-06-04T22:00:00Z",
  "local": "2026-06-05T00:00:00+02:00"
}
```

### Advice

- For agents that care about the *user's* day, prefer passing explicit `YYYY-MM-DD` strings computed in the agent's environment (or from user input).
- "today" inside a long-running agent session may cross midnight.
- The parser is intentionally simple and local-first; it does not use the user's configured "home" timezone if the process runs elsewhere.

## Money Handling

- All prices are stored as **integer cents** in the `price_cents` column (`INTEGER`, nullable).
- Input via `--price` is parsed with a tiny tolerant parser:
  - Optional leading `$`
  - `f64` parse
  - `round(val * 100.0)` → i64
  - Must be finite and >= 0
- Output:
  - JSON: `price_cents: 349` and a convenience `price: "$3.49"` (or null)
  - Reports: `cents` + `amount: "$87.45"`
- Internal helper `cents_to_str` always produces two decimal places, no currency symbol except the `$` prefix in formatted strings.

**Never** perform money math in floating point in agent code if you can avoid it — use the cent integers.

## Database

### Location & Lifecycle

- Default: `$XDG_DATA_HOME/nutlog/nutlog.db` (or `~/.local/share/nutlog/nutlog.db`)
- Created on first `open_db` call.
- Parent directory created automatically.
- Migrations run automatically via `PRAGMA user_version` tracking. Never edit the DB schema by hand after the fact.

### Migrations (Current)

v1 — all core tables (products, nutrients, tags, stores, purchases, consumptions, product_nutritions, product_micronutrients) + indexes.

v2 — idempotent `INSERT OR IGNORE` of the 10 standard nutrients.

Adding new migrations follows the pattern in `src/db.rs`: append a string to the `migrations` array, bump the version logic.

### Schema Highlights (for understanding, not for writing raw SQL)

Key tables (simplified):

```sql
products (id, name, created_at, updated_at)
nutrients (id, name UNIQUE, unit, recommended_intake, created_at)
product_tags (id, name UNIQUE, created_at)
product_tag_associations (product_id, tag_id)  -- composite PK, cascades
stores, store_tags, store_tag_associations (similar)
purchases (id, product_id REFERENCES products ON DELETE RESTRICT,
           quantity, price_cents, store_id REFERENCES stores ON DELETE SET NULL,
           purchased_at, created_at)
consumptions (id, product_id REFERENCES ... ON DELETE CASCADE, quantity, unit,
              consumed_at, created_at)
product_nutritions (product_id PK REFERENCES ... CASCADE, reference_*, macros...)
product_micronutrients (id, product_id, nutrient_id REFERENCES nutrients ON DELETE CASCADE,
                        amount, unit) UNIQUE(product, nutrient)
```

Foreign keys are enforced (`PRAGMA foreign_keys = ON`).

### Delete behavior (CLI)

| Entity        | Command                    | Notes |
|---------------|----------------------------|-------|
| `product`     | `product delete`           | Blocked if purchases exist unless `--force` (deletes purchases first) |
| `nutrient`    | `nutrient delete`          | Blocked if referenced in `product_micronutrients` unless `--force` |
| `purchase`    | `purchase delete`          | Unconditional |
| `consumption` | `consumption delete`       | Unconditional |
| `store`       | `store delete`             | Unconditional; purchases get `store_id` set to NULL |
| `product-tag` | `product-tag delete`       | Unconditional; associations cascade |
| `store-tag`   | `store-tag delete`         | Unconditional; associations cascade |

### Timestamps

- All `*_at` columns are `TEXT` containing RFC3339 UTC strings (seconds precision).
- `created_at` on most rows is set at insert time via `now_utc()`.
- `purchased_at` / `consumed_at` come from the user-supplied (or defaulted) `--date`, normalized as described above.
- `products.updated_at` is only bumped on `rename` currently.

### Why No ORM?

The project deliberately uses raw `rusqlite` + small query builders for predictability and ease of review by agents. All queries are in one place (`src/main.rs` inside the commands module).

## JSON Serialization Details

- Most structs derive `Serialize` (and sometimes `Deserialize` for input shapes).
- Optional numeric fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- `micronutrients` defaults to empty vec on serialize.
- `price` / `amount` human strings are added in the command handlers, not in the DB models.
- `Success` and the anonymous error wrapper are constructed on the fly.

## Internal Implementation Notes (for Curious Agents)

- Fuzzy ranking happens in Rust with `strsim::jaro_winkler`; results are re-fetched as full rows afterwards.
- Report nutrition scaling and micro aggregation happen entirely in Rust after fetching the raw rows (no complex SQL).
- The `commands` module is defined inside `main.rs` for now (keeps the binary simple; can be split later if the file grows too much).
- Error types are `thiserror` enums; only at the very top level does `anyhow` come into play for the `run()` Result.

## Extending the Model

If you need to add a column or table:

1. Add a new migration string in `db.rs`.
2. Update the relevant Rust structs in `models.rs`.
3. Add or modify handlers in `main.rs`.
4. Add `--json` support and help text.
5. Add or update integration tests.
6. Document the change here and in command-reference / nutrition docs.
7. Update `AGENTS.md` / `CODING_PRACTICES.md` if a new pattern is introduced.

See the project guidelines for the required `cargo fmt && cargo clippy -- -D warnings && cargo test` before changes.

## Direct DB Access

You *can* open the SQLite file with any tool (`sqlite3`, DB Browser, etc.). This is useful for:

- Advanced micronutrient entry
- Custom reporting / exports
- Data recovery

Just remember:
- Respect the foreign keys and cascades.
- Timestamps must be valid RFC3339 UTC.
- Money must be cents (or NULL).
- After manual changes, the tool will still work, but you may confuse reports or future migrations.

Prefer driving everything through the `nutlog` CLI when possible — that is the supported interface.

## Files of Interest in the Source

- `src/db.rs` — path resolution, migration runner, date parsing, timestamp formatting
- `src/models.rs` — all the `Serialize` shapes you will receive under `--json`
- `src/error.rs` — the error vocabulary
- `src/main.rs` — the entire command implementation (search for `handle_` functions)
- `src/cli.rs` — the clap definition (the help text source)

## Summary

- Dates are local-day based, stored as UTC instants, shown in local.
- Money = integer cents everywhere.
- Schema is small, explicit, and documented by the code + this file.
- The tool is intentionally "dumb but reliable" — agents are expected to be smart.

This keeps the core simple and the agent layer powerful.
