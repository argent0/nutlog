# Getting Started

This guide walks through a minimal session using `nutlog`.

## 1. First Run & Database

The first command you run will initialize everything:

```bash
nutlog --json nutrient list
```

- Creates the default database if missing (see below).
- Runs any pending migrations.
- Pre-populates 10 common nutrients (Protein, Carbohydrates, Fat, Fiber, Sugars, Vitamin C, Vitamin D, Calcium, Iron, Potassium).

## Database Location

Default (follows [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)):

```
$XDG_DATA_HOME/nutlog/nutlog.db
# usually:
~/.local/share/nutlog/nutlog.db
```

Override for a specific session or script:

```bash
nutlog --db /tmp/test-nutlog.db --json product list
```

The parent directory is created automatically.

> **Tip for agents**: Always pass `--db` explicitly in scripts or when you want an isolated/test database. Never rely on the default path in automated flows unless you control the environment.

## 2. Create Your First Product

```bash
nutlog product create "Greek Yogurt 170g - Plain" --tags yogurt,dairy,protein
```

Output (human):

```
Created product 1 (Greek Yogurt 170g - Plain)
```

With JSON:

```bash
nutlog --json product create "Greek Yogurt 170g - Plain" --tags yogurt,dairy,protein
```

```json
{
  "success": true,
  "id": 1,
  "message": "Created product 1 (Greek Yogurt 170g - Plain)"
}
```

## 3. Add Nutritional Information

Nutrition is attached to a product and expressed **per reference amount** (e.g. per 100g or per container).

```bash
nutlog product nutrition set 1 \
  --reference-quantity 100 \
  --reference-unit g \
  --energy-kcal 97 \
  --protein-g 9.0 \
  --carbohydrates-g 3.8 \
  --fat-g 5.0 \
  --fiber-g 0
```

Human confirmation or JSON success object.

See [Nutrition Tracking](nutrition-tracking.md) for the full model (micronutrients and supplement actives are set with `--micronutrient` or `--json-file` on `product nutrition set`).

## 4. Record a Purchase

```bash
nutlog purchase create 1 \
  --price 1.49 \
  --store 1 \           # optional; create a store first if you want
  --quantity 4 \
  --date yesterday
```

- `--price` accepts `1.49` or `$1.49` (internally stored as 149 cents).
- `--date` is very flexible (see [data-model](data-model.md)).
- Quantity defaults to 1.0.

## 5. Log Consumption

Consumption records *what you actually ate* (distinct from purchases).

```bash
nutlog consumption create 1 \
  --quantity 170 \
  --unit g \
  --date today
```

If you omit `--quantity` and `--unit`, and the product has a reference nutrition amount set, it will fall back to that reference quantity (useful for "one serving").

## 6. View Data

List products (human table or JSON):

```bash
nutlog product list
nutlog --json product list
```

Search (fuzzy by name or exact tag):

```bash
nutlog --json product search --name "yogurt"
nutlog --json product search --tag dairy
```

Show full details (tags + nutrition):

```bash
nutlog product show 1
nutlog --json product show 1
```

Purchases and consumption support `--since` / `--until` / `--product` / `--store` filters.

## 7. Generate a Report

```bash
nutlog --json report nutrition --since 2026-05-01 --until 2026-05-31
nutlog report spending --by store --since yesterday
```

See [Reporting](reporting.md).

## 8. Using Tags

Tags are lightweight taxonomies for products and stores. They are created on first use in many places.

```bash
nutlog product tag add 1 --tag organic
nutlog store create "Whole Foods"
nutlog store tag add 1 --tag supermarket
nutlog product-tag list
nutlog store-tag search "super"
```

Tags can be deleted (associations are removed).

Individual purchase and consumption records can also be deleted by ID:

```bash
nutlog purchase delete 42
nutlog consumption delete 5
```

Custom nutrients can be removed with `nutrient delete <id>` (use `--force` if products still reference them).

## Next

- Full command syntax and every flag: [Command Reference](command-reference.md)
- How nutrition math and scaling works: [Nutrition Tracking](nutrition-tracking.md)
- Using from code/LLMs: [Agent & JSON Usage](agent-usage.md)
- Flexible date parsing rules: [data-model](data-model.md)

## Minimal Productive Workflow (Humans)

1. `nutlog product create "..." --tags foo,bar`
2. `nutlog product nutrition set ID --reference-quantity 100 --reference-unit g --protein-g ...`
3. `nutlog store create "My Store"` (optional)
4. `nutlog purchase create ID --price 3.99 --store 1 --date today`
5. `nutlog consumption create ID --quantity 150 --unit g`
6. `nutlog --json report nutrition --since "last week"`

## Minimal Workflow (Agents / Scripts)

Always use `--json`, pass `--db` explicitly when possible, parse the returned JSON objects, and handle the `success: false` error envelope.

See the dedicated agent guide.
