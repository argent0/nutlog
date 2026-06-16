# Command Reference

This document describes every command, subcommand, flag, and argument in detail.

**The CLI `--help` output is the single source of truth for exact syntax.** This reference adds explanations, examples, return values, and agent notes.

All commands accept the global flags `--json`, `--db <PATH>`, `--quiet`.

## Global Flags

| Flag       | Description                                      | Notes for Agents                  |
|------------|--------------------------------------------------|-----------------------------------|
| `--json`   | Structured pretty-printed JSON output            | Always use this for scripting     |
| `--db`     | Full path to SQLite file (creates dirs)          | Recommended for isolation         |
| `--quiet`  | Suppress human progress / success messages       | Useful with agents that parse stdout |
| `--help`   | Show help                                        | -                                 |
| `--version`| Show version                                     | -                                 |

## Entities Overview

| Entity         | Purpose                              | Typical Actions                     |
|----------------|--------------------------------------|-------------------------------------|
| `product`      | Food items you buy and eat           | create, list, search, show, rename, tag, delete, nutrition |
| `nutrient`     | Master list of nutrient definitions  | list, create, show, search, delete    |
| `product-tag`  | Taxonomy for products                | create, list, search, show, delete  |
| `purchase`     | Purchase events (with price, qty)    | create, list, show, delete          |
| `store`        | Shopping locations                   | create, list, show, rename, tag, delete |
| `store-tag`    | Taxonomy for stores                  | create, list, search, show, delete  |
| `consumption`  | Actual eaten amounts + dates         | create, list, delete                |
| `report`       | Derived summaries                    | nutrition, spending                 |

## product

Manage the core food catalog.

### product create

```bash
nutlog product create <NAME> [--tags TAGS]
```

- `<NAME>`: free text, e.g. brand + size + flavor. Stored exactly as given.
- `--tags`: comma-separated list (no spaces around commas recommended). Tags are created if they do not exist.

**JSON success**:

```json
{ "success": true, "id": 42, "message": "Created product 42 (...)" }
```

**Examples**:

```bash
nutlog product create "Oats 500g Rolled" --tags oats,breakfast
nutlog --json product create "Milk 1L Semi" --tags dairy
```

### product list

```bash
nutlog product list
```

- Newest first (by id descending).
- Human: table with ID, Name, Tags.
- JSON: array of full `Product` objects (includes tags, nutritional_information, created/updated timestamps in both utc + local).

Empty list prints "(no products)" in human mode.

### product search

```bash
nutlog product search [--name QUERY] [--tag TAG]
```

- `--name`: fuzzy search (Jaro-Winkler) over product names. Returns up to ~50 ranked results.
- `--tag`: exact match on a tag name (case sensitive as stored). No fuzzy on tag filter currently.
- No arguments: behaves like `list`.
- Results include full product objects in JSON (same shape as list).

Human output is a simple one-line summary per match.

### product show <ID>

Displays:

- Name, tags (or "(none)")
- Nutrition block if set (reference amount + macros + first few micronutrients)
- Created timestamp (local)

JSON returns the complete `Product` with `nutritional_information` (or null).

Error if not found (non-zero exit, JSON error envelope).

### product rename <ID> --name "New Name"

Updates name and `updated_at`.

JSON: `{ "success": true, "message": "..." }` (no id on rename).

### product tag

Subcommands:

- `add <ID> --tag NAME` — creates tag if missing, attaches (idempotent).
- `remove <ID> --tag NAME` — detaches if present (no error if missing).

Success objects on both.

### product delete <ID> [--force]

- Without `--force`: fails (non-zero) if any purchases reference the product. Error message suggests `--force`.
- With `--force`: deletes purchases for the product first, then the product (and its nutrition rows via FK cascade).
- Also removes tag associations.

**JSON error example** (without force when purchases exist):

```json
{ "success": false, "error": "product 5 has associated purchases; use --force to delete anyway" }
```

### product nutrition set <ID>

```bash
nutlog product nutrition set <ID> \
  --reference-quantity <QTY> --reference-unit <UNIT> \
  [--energy-kcal N] [--protein-g N] ... [--sugars-g N] \
  [--micronutrient NAME AMOUNT UNIT ...]
```

- Replaces (upserts) the nutrition facts for the product.
- `--reference-quantity` and `--reference-unit` are required unless you use `--json-file` (which must contain its own `reference` object).
- Micronutrients and active compounds are supplied with the repeatable `--micronutrient` flag (three values: `NAME AMOUNT UNIT`):

```bash
nutlog product nutrition set 13 \
  --reference-quantity 1 --reference-unit capsule \
  --micronutrient "Omega 3 EPA" 181 mg \
  --micronutrient "Omega 3 DHA" 121 mg \
  --micronutrient "Creatine Monohydrate" 5 g
```

- A JSON payload can be supplied instead (or for very complex cases):

```bash
nutlog product nutrition set 13 --json-file nutrition.json
```

  The file must include at least a `reference`; macros and `micronutrients` (array of `{name, amount, unit}`) are optional. Example:

  ```json
  {
    "reference": { "quantity": 1.0, "unit": "capsule" },
    "micronutrients": [
      { "name": "Omega 3 EPA", "amount": 181, "unit": "mg" }
    ]
  }
  ```

- A `set` call is authoritative: the micronutrients present after the call are exactly those supplied (omitted ones are removed). The same is true for the macro values.
- Nutrient names are resolved case-insensitively. Unknown names are created automatically (the supplied unit becomes the nutrient's canonical unit; recommended intake is left blank).
- No unit conversion is performed at set or report time (consumer qty and ref qty assumed compatible, e.g. both g or both ml).

**Success**: simple `{ "success": true, "message": "Nutrition set for product N" }`

See [nutrition-tracking.md](nutrition-tracking.md) for the full data shape, JSON output examples, and scaling rules. `product show --json` includes the complete `nutritional_information` (macros + `micronutrients` array with `nutrient_id`, `name`, `amount`, `unit`).

## nutrient (master data)

Pre-populated list + user extensions.

### nutrient list

Lists all (name order). Human shows `id: Name (unit rec:XX.X)`.

JSON array of `Nutrient`.

The 10 built-ins have fixed IDs on a fresh DB (order of INSERT OR IGNORE).

### nutrient create <NAME> --unit <U> [--recommended-intake AMOUNT]

Creates a new nutrient definition. Name must be unique (DB constraint).

Recommended intake is optional (daily value etc.).

### nutrient show <ID>

JSON or debug print of the row.

### nutrient search <QUERY>

Fuzzy (Jaro-Winkler) search over nutrient names. Returns full objects ranked best-first.

### nutrient delete <ID> [--force]

- Without `--force`: fails (non-zero) if any `product_micronutrients` rows reference the nutrient. Error message suggests `--force`.
- With `--force`: deletes the nutrient (and its product micronutrient associations via FK cascade).
- Unreferenced nutrients (including custom ones) delete without `--force`.

**JSON error example** (without force when referenced):

```json
{ "success": false, "error": "nutrient 17 is referenced by product nutrition data; use --force to delete anyway" }
```

## product-tag

Lightweight controlled vocabulary for products.

Actions: `create <name>`, `list`, `search <query>`, `show <id>`, `delete <id>`

- `list` and `show` include `usage_count` (how many products use the tag).
- `delete` removes the tag and all associations (no check for usage).
- `search` is fuzzy on name.
- `create` is idempotent in effect (INSERT OR IGNORE) but still returns the id of the (existing or new) tag.

## purchase

### purchase create <PRODUCT_ID>

Options:

- `--price <PRICE>` : "4.99", "$19.99" etc. Stored as integer cents. Optional.
- `--store <ID>` : optional FK to stores (validated).
- `--date <DATE>` : flexible, defaults to "today". See date parsing rules.
- `--quantity <Q>` : float, defaults to 1.0.

Validates that product and (if given) store exist.

**JSON on success**: `{ "success": true, "id": <purchase_id>, "message": "..." }`

### purchase list [--since D] [--until D] [--product ID] [--store ID]

- Filters are ANDed.
- Ordered by purchased_at DESC, then id DESC.
- Human: nice table (Date, Product, Qty, Price, Store).
- JSON: array of `Purchase` (includes denormalized product_name, store_name, price as both cents and formatted string).

Missing price shows as null / "-".

### purchase show <ID>

Full single purchase record or "not found".

### purchase delete <ID>

- Unconditional delete by purchase ID.
- Does not affect the product or store; only removes the purchase row.

**JSON success**: `{ "success": true, "message": "Deleted purchase N" }`

## store

Simple catalog of locations.

Actions: `create <name>`, `list`, `show <id>`, `rename <id> --name`, `tag {add,remove}`, `delete <id>`

- `list` and `show` include joined tags (comma list or array).
- `delete` is unconditional (purchases are only SET NULL on store_id).
- Tag add/remove same pattern as products.

## store-tag

Identical shape to `product-tag` but for stores. Includes usage_count of stores.

## consumption

### consumption create <PRODUCT_ID>

```bash
nutlog consumption create <PRODUCT_ID> [--quantity Q] [--unit U] [--date D]
```

- If `--quantity` omitted, falls back to the product's reference quantity (if nutrition info has been set for it), otherwise 1.0.
- `--unit` is free text (g, ml, "serving", etc.). Stored as-is; no conversion.
- Date flexible, defaults to today.

**Important**: Consumption does **not** decrease inventory. There is no inventory concept.

### consumption list [--since] [--until] [--product]

Simple list or JSON array of `Consumption` records (with product_name denormalized).

### consumption delete <ID>

- Unconditional delete by consumption ID.
- Does not affect the product; only removes the consumption row.

**JSON success**: `{ "success": true, "message": "Deleted consumption N" }`

## report

### report nutrition [--since D] [--until D]

Computes totals by looking at all consumption records in the (optional) date range that have corresponding `product_nutritions` rows.

For each consumption:

- `scale = consumed_qty / reference_qty`
- Add `macro_value * scale` to totals.
- Also aggregates scaled micronutrients (if any were recorded on the product).

Output shape (`NutritionReport`):

```json
{
  "period": { "since": "...", "until": "..." },
  "total_consumed_items": 12,
  "totals": {
    "energy_kcal": 1234.5,
    "protein_g": 67.8,
    ...
  },
  "micronutrients": [
    { "nutrient_id": 7, "name": "Vitamin D", "unit": "µg", "total_amount": 12.3 }
  ]
}
```

- Human output is abbreviated (shows main macros + top 5 micros).
- Only products that have nutrition data contribute. Others are silently ignored for the report.
- No unit conversion (see nutrition doc).
- `total_consumed_items` counts only the consumptions that had nutrition data.

### report spending [--by total|store|product] [--since] [--until] [--period ...]

- Always computes overall total (even when grouping).
- `--by store` (default behavior in code): populates `by_store` array (sorted by spend desc). Includes "(no store)" bucket.
- `--by product`: also populates `by_product`.
- Other `--by` or `--period` values are accepted by CLI but currently ignored beyond the basic grouping (the implementation focuses on total + store + optional product).

`SpendingReport` JSON:

```json
{
  "period": {...},
  "total_cents": 12345,
  "total": "$123.45",
  "by_store": [ { "store_id": 3, "store_name": "...", "cents": 4500, "amount": "$45.00", "purchase_count": 5 }, ... ],
  "by_product": [ ... ]   // only present when --by product
}
```

Human prints a short summary.

## Error Handling (All Commands)

- On success (mutating): 0 exit, JSON has `"success": true`
- On failure (validation, not found, etc.): non-zero exit.
  - Human: message on stderr
  - JSON: stdout contains `{ "success": false, "error": "human readable reason" }`

Common errors:

- `product not found: 99`
- `store not found: 5`
- `product 7 has associated purchases; use --force to delete anyway`
- `nutrient 17 is referenced by product nutrition data; use --force to delete anyway`
- `invalid price: abc`
- `invalid date: ...`
- `unrecognized date format: 'foo'`

See `src/error.rs` for the full list.

## JSON Object Shapes (Reference)

See [models.rs](../src/models.rs) for the canonical Rust structs that are serialized.

Key top-level ones:

- `Product`, `NutritionalInformation`, `ReferenceAmount`, `Micronutrient`
- `Nutrient`
- `Tag`
- `Store`
- `Purchase`
- `Consumption`
- `NutritionReport` / `MacroTotals` / `MicroTotal`
- `SpendingReport` / `StoreSpending` / `ProductSpending`
- `Success` and the error envelope

Timestamps are always:

```json
{ "utc": "2026-06-05T12:34:56Z", "local": "2026-06-05T14:34:56+02:00" }
```

## Consistency Rules

- `create` subcommands for tags are mostly idempotent (INSERT OR IGNORE) and return the id.
- Search results for name are always sorted by similarity desc.
- List for purchases/consumption is newest first.
- All monetary output in JSON includes both `price_cents` (or `cents`) **and** a formatted `price` / `amount` string for humans.

## See Also

- [getting-started.md](getting-started.md) for common flows
- [agent-usage.md](agent-usage.md) for machine consumption patterns
- `nutlog <subcommand> --help` for the latest flag text
