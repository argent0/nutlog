# Reporting

`nutlog` provides two built-in reports under the `report` entity. Reports are derived views — they never modify data.

Both reports support `--since` and `--until` date filters using the same flexible date parser as purchases and consumption.

Use `--json` to get complete, machine-readable structures suitable for further processing or LLM consumption.

## report nutrition

Nutrition reports have two subcommands: `summary` (period totals) and `list` (per-day breakdown).

Shared date flags (both subcommands):

- `--since DATE` — start of period (inclusive)
- `--until DATE` — end of period (inclusive)
- `--days N` — last N calendar days inclusive of today (cannot be combined with `--since`/`--until`)

### report nutrition summary

```bash
nutlog report nutrition summary [--since DATE] [--until DATE] [--days N]
nutlog --json report nutrition summary --since "2026-05-01" --until "2026-05-31"
nutlog --json report nutrition summary --days 7
```

#### What it calculates

- Finds every consumption record whose `consumed_at` is inside the window (inclusive on both ends when provided).
- For each such record that has a matching row in `product_nutritions`, scales the stored nutrition facts by `consumed_qty / ref_qty`.
- Sums the scaled values across all qualifying consumptions.
- Also scales and sums any micronutrients attached to those products.

#### Output

Human mode prints a short summary (main macros + up to 5 micros).

JSON mode returns a `NutritionReport`:

```json
{
  "period": {
    "since": "2026-05-01",
    "until": "2026-05-31",
    "days": null
  },
  "total_consumed_items": 7,
  "totals": {
    "energy_kcal": 1850.0,
    "protein_g": 78.5,
    "carbohydrates_g": 210.0,
    "fat_g": 65.2,
    "fiber_g": 22.0,
    "sugars_g": 45.0
  },
  "micronutrients": [
    {
      "nutrient_id": 7,
      "name": "Vitamin D",
      "unit": "µg",
      "total_amount": 8.4
    }
  ]
}
```

- `total_consumed_items`: number of consumption rows that contributed nutrition data (not the number of distinct products).
- Any macro or micro that had no data across the period will be absent or null in the `totals` object (see serde skip rules).
- Micronutrients array is always present (may be empty).

#### Filtering behavior

- No `--since` → from the beginning of time.
- No `--until` → up to now.
- Both omitted → all consumption that has nutrition attached.
- `--days N` → overrides `--since`/`--until` with a rolling window ending today.

### report nutrition list

```bash
nutlog report nutrition list [--since DATE] [--until DATE] [--days N] [--value VALUE]
nutlog --json report nutrition list --days 7 --value protein
nutlog report nutrition list --value macronutrients --since 2026-05-01 --until 2026-05-31
```

#### `--value` options

| Value | Shows per day |
|-------|---------------|
| `macronutrients` (default) | energy, protein, carbohydrates, fat, fiber, sugars |
| `calories` | energy (kcal) only |
| `protein` | protein (g) only |
| `carbohydrates` | carbohydrates (g) only |
| `fat` | fat (g) only |
| `fiber` | fiber (g) only |
| `sugars` | sugars (g) only |

Micronutrients are not included in `list` output.

#### Output

Human mode prints a compact header-underlined table (one row per day), same style as repslog:

- `--value macronutrients` (default): columns `Date`, `Energy (kcal)`, `Protein (g)`, `Carbs (g)`, `Fat (g)`, `Fiber (g)`, `Sugars (g)`, `Items`
- Single-macro `--value`: columns `Date`, the selected nutrient with unit in the header, `Items`

JSON mode returns a `NutritionDailyReport`:

```json
{
  "period": {
    "since": "2026-06-28",
    "until": "2026-07-04",
    "days": 7
  },
  "value": "protein",
  "days": [
    {
      "date": "2026-06-28",
      "total_consumed_items": 2,
      "totals": { "protein_g": 45.0 }
    },
    {
      "date": "2026-06-29",
      "total_consumed_items": 0,
      "totals": {}
    }
  ]
}
```

- When the period is bounded (`--days`, both `--since` and `--until`, or `--since` only), every calendar day in the range is included with zero-filled totals for days without consumption.
- When no date filter is given, only days that have consumption are listed (sparse mode).

Dates are interpreted with the same rules as everywhere else (see [data-model](data-model.md)).

### Limitations & Gotchas

- Only consumption entries linked to products that have had `nutrition set` contribute.
- No automatic unit conversion (g vs ml, etc.).
- If you eat 0 quantity or reference quantity was 0, that row contributes 0.
- Micronutrients from products without base macros still get counted if the join finds the nutrition row? Wait — the query joins on `product_nutritions`, so a product must have the base row even if only micros were added later. (Current implementation requires the base row for any contribution.)

## report spending

```bash
nutlog report spending [--by total|store|product] [--since D] [--until D]
nutlog --json report spending --by store --since "last month"
nutlog --json report spending --by product
```

### What it calculates

- Total money spent on recorded purchases in the window (sum of `price_cents`, treating null prices as 0).
- Breakdown by store (always included).
- Breakdown by product when `--by product` is passed.

Purchases without a price contribute 0 to totals.

### Output

JSON `SpendingReport`:

```json
{
  "period": { "since": null, "until": null },
  "total_cents": 8745,
  "total": "$87.45",
  "by_store": [
    {
      "store_id": 2,
      "store_name": "Supermercado XYZ",
      "cents": 5499,
      "amount": "$54.99",
      "purchase_count": 3
    },
    {
      "store_id": null,
      "store_name": "(no store)",
      "cents": 3246,
      "amount": "$32.46",
      "purchase_count": 5
    }
  ],
  "by_product": [   /* present only with --by product */
    {
      "product_id": 1,
      "product_name": "Greek Yogurt 170g - Plain",
      "cents": 596,
      "amount": "$5.96",
      "purchase_count": 4
    }
  ]
}
```

Human mode prints the total and the by-store list (and by-product if requested).

### Grouping notes

- `--by total` (or omitted) still populates `by_store`.
- `--by product` adds the product breakdown (can be expensive on huge histories; usually fine).
- The `--period` flag is accepted by the parser but is currently not used for additional bucketing in the implementation. It is reserved for future grouping (e.g. monthly totals).

### Edge cases

- No purchases in range → `total_cents: 0`, `total: "$0.00"`, empty by_store array.
- Purchases with `store_id = NULL` are grouped under `"(no store)"`.
- Price is optional on purchase; those rows add 0 to money totals but still count in `purchase_count` for the store/product.

## Using Reports from Agents

Always request `--json`.

Typical agent flow:

1. Decide on a period (e.g. last 7 days, this month).
2. Call `report nutrition summary --json --since ...` or `report nutrition list --json --days 7 --value protein`
3. Parse the `totals` and decide whether to log more consumption or surface a summary to the user.
4. Optionally call spending report for budgeting context.

Reports are read-only and relatively cheap (simple aggregates over indexed columns).

## Combining with Other Commands

Example: "What did I spend on yogurt last month?"

```bash
# 1. find the product id(s)
nutlog --json product search --name yogurt --tag dairy

# 2. spending filtered by product (client-side or future server-side)
nutlog --json report spending --by product --since "last month"
# then filter the JSON array client-side for the ids of interest
```

Currently there is no `--product` filter directly on the spending report (you get everything or by-product global). Post-filter in your agent code.

## Future Directions (Not Yet Implemented)

- More grouping modes for spending (`--by month`, `--period month`)
- Weekly/monthly rollups beyond daily list
- Export of reports to CSV / other formats
- Budget targets vs actual (would live outside core reports)

## See Also

- [command-reference.md](command-reference.md) — exact flag names for the two report subcommands
- [nutrition-tracking.md](nutrition-tracking.md) — how the underlying numbers are produced
- `src/main.rs` (handle_report) for the exact SQL and scaling logic if you need to understand edge cases
