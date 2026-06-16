# Nutrition Tracking

`nutlog` stores nutritional facts at the **product** level and derives intake from **consumption** records.

## Core Model

Each product that has nutrition data has exactly one row in `product_nutritions`:

- `reference_quantity` + `reference_unit` — e.g. `100`, `g` or `1`, `serving` or `170`, `g`
- Macro columns (all optional): `energy_kcal`, `protein_g`, `carbohydrates_g`, `fat_g`, `fiber_g`, `sugars_g`

Additionally, zero or more rows in `product_micronutrients`:

- FK to a `nutrient` (from the master `nutrients` table)
- `amount` + `unit` (unit is stored per value; no normalization)

The master `nutrients` table defines the vocabulary (name + canonical unit + optional recommended daily intake).

## Setting Nutrition via CLI

The `product nutrition set` command accepts the reference amount plus macros. It now also supports micronutrients and active compounds directly:

```bash
nutlog product nutrition set 13 \
  --reference-quantity 1 --reference-unit capsule \
  --energy-kcal 10 \
  --micronutrient "Omega 3 EPA" 181 mg \
  --micronutrient "Omega 3 DHA" 121 mg \
  --micronutrient "Creatine Monohydrate" 5 g
```

- `--micronutrient` is repeatable. Each use takes three values: `NAME AMOUNT UNIT`.
- Unknown nutrient names are created automatically (with the supplied unit as their canonical unit and no recommended intake). Existing nutrients are matched case-insensitively by name.
- For complex cases or machine-generated data you can also use:

```bash
nutlog product nutrition set 13 --json-file nutrition.json
```

Example `nutrition.json` (all macro fields and the micronutrients array are optional):

```json
{
  "reference": { "quantity": 1.0, "unit": "capsule" },
  "energy_kcal": 10,
  "micronutrients": [
    { "name": "Omega 3 EPA", "amount": 181, "unit": "mg" },
    { "name": "Omega 3 DHA", "amount": 121, "unit": "mg" }
  ]
}
```

A `set` call fully replaces the nutrition facts for the product (both the macro row and the exact set of micronutrients). Previously set micronutrients that are omitted are removed.

## How Reports Scale Consumption

When `report nutrition` runs:

1. It joins consumption records (in the date filter) to `product_nutritions`.
2. For every such pair it computes:

   ```
   scale = consumption.quantity / product_nutritions.reference_quantity
   ```

3. Each macro value is multiplied by the scale and summed.
4. The same scaling is applied to every micronutrient row belonging to that product.
5. Products without a nutrition row contribute 0 to the report and are not counted in `total_consumed_items`.

**Important limitations** (documented so agents don't assume magic):

- **No unit conversion**. If you record consumption in `ml` but the reference is `g`, the numbers will be wrong. Keep units consistent (both mass or both volume) per product.
- Scale can be > 1 or < 1. A 200 g consumption against a 100 g reference yields 2× the nutrients.
- Negative quantities are not prevented at input time (though not useful).
- Micronutrient totals only include nutrients that were explicitly stored on the consumed products.

## Reference Amount Examples

Common patterns:

- Per 100 g (most packaged foods in Europe)
- Per 1 serving / per container (US style)
- Per 100 ml (beverages)
- Per piece / per unit (fruit, eggs, bars)

Choose whatever matches the numbers on the label you are copying from.

When logging consumption, use the **actual weight/volume** you ate, not "1 serving". The tool will do the math.

Example:

Product reference: 100 g → 8 g protein

You eat 150 g → report will show 12 g protein from that consumption.

## Viewing Nutrition on a Product

```bash
nutlog product show 42
nutlog --json product show 42
```

JSON shape excerpt:

```json
{
  "id": 42,
  "name": "...",
  "tags": ["dairy"],
  "nutritional_information": {
    "reference": { "quantity": 100.0, "unit": "g" },
    "energy_kcal": 97.0,
    "protein_g": 9.0,
    "carbohydrates_g": 3.8,
    "fat_g": 5.0,
    "fiber_g": null,
    "sugars_g": null,
    "micronutrients": [
      { "nutrient_id": 7, "amount": 1.2, "unit": "µg", "name": "Vitamin D" }
    ]
  },
  "created_at": { "utc": "...", "local": "..." },
  "updated_at": { "utc": "...", "local": "..." }
}
```

`nutritional_information` is `null` when no data has been set.

## Pre-populated Nutrients

On first database creation the following are inserted (with reasonable default recommended intakes where known):

| Name          | Unit | Recommended |
|---------------|------|-------------|
| Protein       | g    | 50          |
| Carbohydrates | g    | 300         |
| Fat           | g    | 70          |
| Fiber         | g    | 25          |
| Sugars        | g    | (null)      |
| Vitamin C     | mg   | 90          |
| Vitamin D     | µg   | 15          |
| Calcium       | mg   | 1000        |
| Iron          | mg   | 18          |
| Potassium     | mg   | 4700        |

You can add more with `nutlog nutrient create "Vitamin B12" --unit µg --recommended-intake 2.4`

## Data Integrity

- `product_nutritions.product_id` has ON DELETE CASCADE.
- `product_micronutrients` rows are also cascaded when product is deleted.
- `nutrient delete <id>` fails if any product micronutrient rows reference the nutrient, unless `--force` is used (which cascades those rows).

## Tips for Accurate Tracking

1. Set nutrition data once per product (update when formulation changes).
2. Be consistent with units on a per-product basis.
3. Use real measured consumption quantities when possible (kitchen scale).
4. For composite meals you can create a "virtual" product that represents the whole meal and set aggregate nutrition for it.
5. Reports only see what you have both consumed *and* described nutritionally.

## Micronutrient Notes for Agents

Micronutrients (and supplement actives) are now first-class via the CLI:

- Use `--micronutrient "Name" AMOUNT UNIT` (repeatable) on `product nutrition set`.
- Or supply a complete payload with `--json-file nutrition.json` (see examples above).
- New nutrient names supplied this way are created automatically.
- Use `nutrient list --json` (or search) when you want stable IDs or to pre-create entries with a recommended intake.
- `product show --json` and `report nutrition --json` both surface the full micronutrient data (reports scale them exactly like macros).

The simple macro-only path remains fast for ordinary foods; the new flags unlock the full existing data model for supplements without requiring raw SQL.

## See Also

- [command-reference.md](command-reference.md) — the `product nutrition set` syntax
- [reporting.md](reporting.md) — how the numbers appear in reports
- `src/models.rs` — `NutritionalInformation`, `Micronutrient` structs
- `src/main.rs` — `set_nutrition` and the scaling logic inside `handle_report`
