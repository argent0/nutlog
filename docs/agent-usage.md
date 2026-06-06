# LLM Agent & JSON / Scripting Usage

`nutlog` was designed from the ground up to be used primarily by LLM agents (or other automation) on behalf of a single user.

The `--json` flag is the primary interface for agents.

## Core Contract for Agents

1. **Always pass `--json`** when you want structured data or reliable success/failure signaling.
2. **Check the exit code** — non-zero always means failure.
3. **Parse JSON from stdout** on both success and error paths.
4. **Success envelope** for mutations:

   ```json
   { "success": true, "id": 123, "message": "Created product 123 (...)" }
   ```

   Some mutations return `id: null` when no new row id is relevant (e.g. rename, tag add).

5. **Error envelope** (stdout + non-zero exit):

   ```json
   { "success": false, "error": "product 5 has associated purchases; use --force to delete anyway" }
   ```

6. Global flags `--db`, `--quiet` are also useful:
   - `--db` lets the agent work against an explicit, known database path (highly recommended).
   - `--quiet` reduces noise when the agent only cares about the JSON result.

## Recommended Invocation Pattern

```bash
nutlog --json --db "$DB_PATH" product create "..." --tags "a,b"
```

Capture stdout, parse as JSON, inspect `success`.

Never rely on human text parsing unless `--json` is impossible for that command (currently all data-returning commands support it).

## Typical Agent Workflows

### Bootstrapping / Discovery

```bash
# Ensure DB exists and see what nutrients are available
nutlog --json --db "$DB" nutrient list

# See existing products
nutlog --json --db "$DB" product list
```

### Product + Nutrition Setup (once per new food)

1. Create product → capture the returned `id`.
2. (Optional) create/find tags and attach.
3. Call `product nutrition set <id> ...` (with `--micronutrient NAME AMOUNT UNIT` as needed) using the reference amount + values from the label.

### Daily Logging

- Search for product (fuzzy name or tag) to obtain id.
- Record purchase if relevant (price, store, date).
- Record consumption (actual amount eaten that day).

Example JSON creation of consumption:

```json
// after running the command
{ "success": true, "id": 87, "message": "Recorded consumption 87 of product 3 (150 g)" }
```

### Reporting for the User

Call the report commands with a sensible window and present the `totals` or breakdowns.

The agent can decide to summarize, compare to recommended intakes (the master nutrient list has `recommended_intake`), or highlight outliers.

### Cleanup / Maintenance (rare)

- Rename products when brands change formulation or user wants cleaner names.
- Delete test data with `--force` when necessary.
- Tag hygiene via `product-tag` / `store-tag` delete (use sparingly).

## Fuzzy Search Notes

- `product search --name "yog"` and `nutrient search "prot"` use Jaro-Winkler via the `strsim` crate.
- Results are returned best-match first.
- `search` without `--name`/`--tag` falls back to listing everything.
- Tag filters on product search are **exact** (`--tag yogurt`), not fuzzy.
- For tags themselves, `product-tag search` and `store-tag search` are fuzzy.

Agents can implement their own client-side ranking or post-filter if the top-N is insufficient.

## Date Handling for Agents

Use unambiguous forms when possible:

- `"2026-06-05"` (YYYY-MM-DD) — safest
- `"today"`, `"yesterday"`, `"tomorrow"`
- `"3 days ago"`, `"last week"`

The parser lives in `db::parse_flexible_date`. It treats the string relative to the *local* wall date on the machine running `nutlog`, then stores a UTC instant representing local midnight of that day.

For agents running in containers or on servers in different timezones, be aware that "today" is the *server's* today. Pass explicit ISO dates when the user's "today" matters.

See [data-model.md](data-model.md) for full rules.

## Money

- Never send floating point for money to the tool.
- `--price "4.99"` or `"$4.99"` is parsed and rounded to nearest cent internally.
- JSON always returns both `price_cents: 499` (integer) and a convenience `"price": "$4.99"` string.
- Same for report totals (`total_cents` + `total`).

Agents should prefer the `_cents` fields for any arithmetic.

## Error Cases Agents Must Handle

- Entity not found (product, store, purchase, etc.)
- Product has purchases (delete without force)
- Invalid price / invalid date (the exact string the user/agent supplied is echoed)
- Duplicate nutrient name (on create)
- DB constraint errors surface as `database error: ...` (rare if you follow the command contract)

On any error the agent should surface a clear message to the end user and usually stop the current plan step.

## Idempotency Notes

- `product-tag create "foo"` and `store-tag create "foo"` are safe to call repeatedly (they do INSERT OR IGNORE).
- Tag add/remove on products/stores are idempotent in effect.
- Creating the same product name twice is allowed (names are not unique).
- Nutrition set is an upsert (safe to re-apply).

## Working with Multiple Databases

Agents often maintain:

- A production DB (default or well-known path)
- Ephemeral test DBs passed via `--db /tmp/nutlog-test-$$.db`

Always clean up temp DBs after test runs if they are large or numerous.

## Example Agent Session (Pseudo)

```python
# pseudocode
db = "/home/user/.local/share/nutlog/nutlog.db"

def run(*args):
    out = subprocess.check_output(["nutlog", "--json", "--db", db] + list(args))
    return json.loads(out)

p = run("product", "create", "Banana", "--tags", "fruit")
pid = p["id"]

run("product", "nutrition", "set", str(pid),
    "--reference-quantity", "100", "--reference-unit", "g",
    "--energy-kcal", "89", "--carbohydrates-g", "23")

run("consumption", "create", str(pid), "--quantity", "120", "--unit", "g")

report = run("report", "nutrition", "--since", "today")
print("Protein today:", report["totals"].get("protein_g"))
```

## What the Tool Does *Not* Do (Avoid Hallucinating Features)

- No automatic nutritional lookup from web / barcode / LLM knowledge inside the tool.
- No unit conversion.
- No multi-user, no auth, no encryption at rest (the DB is plain SQLite in the user's home).
- No photos, no inventory levels, no meal planning.
- No "edit" for nutrition once set (delete + re-set the product or use direct SQL).
- Reports do not do daily averages or goal tracking yet.

If you need extra behavior, do it in the agent layer (or propose a minimal, well-scoped extension that follows the existing patterns).

## Updating Documentation

If the CLI behavior changes, update the help text in the clap derives first (source of truth), then update these docs and the top-level README examples.

See [AGENTS.md](../AGENTS.md) for the full contributor guidelines when modifying the tool itself.

## Summary for LLM System Prompts

You can copy/adapt the following into an agent's system prompt or skill description:

> You have access to the `nutlog` CLI. It is a local SQLite nutrition logger.
> Always use `--json`. Prefer explicit `--db` paths. Return values are JSON objects with `success`.
> On error you receive a JSON object with `success:false` and a clear `error` string + non-zero exit.
> Use flexible dates like "today", "2026-06-05", "yesterday".
> Money is in cents. Nutrition is scaled from consumption vs product reference amounts.
> Full docs are available in the installed `docs/` directory or the repo.

This design makes `nutlog` unusually pleasant for tool-using LLM agents.
