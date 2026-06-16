# nutlog

`nutlog` is a simple, local, CLI-first tool to log food purchases, track nutrition, and generate basic reports.

It is designed primarily to be used by LLM agents on behalf of a single user.

## Features

- Command structure: `nutlog <entity> <action> [...]`
- Global `--json` for machine-readable output (primary for agents)
- Global `--db <PATH>` to override DB location
- `--quiet` for minimal output
- All monetary values stored as integer cents
- Timestamps in UTC; human output shows local time
- SQLite embedded DB (XDG data dir by default)
- Pre-populated common nutrients
- Fuzzy search for products, nutrients, tags
- Flexible date input: `today`, `yesterday`, `2026-05-20`, `last week`, etc.
- Nutrition scaling for consumption reports
- Safety on delete (`product` and `nutrient` require `--force` when referenced data exists)

## Installation (dev)

```bash
cargo install --path .
# or during dev: cargo run --
```

Binary: `nutlog`

## Database

Default: follows XDG Base Directory spec → `$XDG_DATA_HOME/nutlog/nutlog.db` (usually `~/.local/share/nutlog/nutlog.db`)

Override: `--db /path/to/nutlog.db`

Migrations run automatically on open.

## Global Flags

```bash
nutlog [GLOBAL OPTIONS] <ENTITY> <ACTION> [ARGS]
```

- `--json` : structured JSON output
- `--db <PATH>`
- `--quiet`
- `--help`, `--version`

## Entities

- `product`
- `nutrient`
- `product-tag`
- `purchase`
- `store`
- `store-tag`
- `consumption`
- `report`

## Examples

### Products

```bash
nutlog product create "YOUGURISIMO 300G NATU" --tags yogurt,natural
nutlog --json product list
nutlog --json product search --name "yogu"
nutlog --json product search --tag yogurt
nutlog product show 1
nutlog product rename 1 --name "New Name"
nutlog product tag add 1 --tag organic
nutlog product tag remove 1 --tag organic
nutlog product delete 1          # fails if purchases exist
nutlog product delete 1 --force
```

### Nutrition

```bash
nutlog product nutrition set 1 \
  --reference-quantity 100 \
  --reference-unit g \
  --energy-kcal 123 \
  --protein-g 8.5 \
  --carbohydrates-g 12.3
```

Micronutrients / active compounds (supplements etc.) are set the same way:

```bash
nutlog product nutrition set 13 \
  --reference-quantity 1 --reference-unit capsule \
  --micronutrient "Omega 3 EPA" 181 mg \
  --micronutrient "Creatine Monohydrate" 5 g
```

`nutlog --json product show <id>` includes `nutritional_information` (macros + `micronutrients` array). Reports automatically scale and aggregate them.

### Nutrients (master list)

```bash
nutlog nutrient list
nutlog --json nutrient list
nutlog nutrient create "Vitamin D" --unit µg --recommended-intake 15
nutlog nutrient show 7
nutlog nutrient search "vit d"
nutlog nutrient delete 17          # fails if products reference it
nutlog nutrient delete 17 --force
```

Common nutrients (Protein, Carbohydrates, Fat, Fiber, Sugars, Vitamin C, Vitamin D, Calcium, Iron, Potassium) are pre-populated.

### Product Tags

```bash
nutlog product-tag create "yogurt"
nutlog product-tag list
nutlog --json product-tag search "yo"
nutlog product-tag show 1
nutlog product-tag delete 1
```

### Purchases

```bash
nutlog purchase create 1 \
  --price 4.99 \
  --store 1 \
  --date yesterday \
  --quantity 2
```

- `--quantity` defaults to 1
- `--price` accepts "19.99" or "$19.99"
- `--date` flexible natural language

```bash
nutlog --json purchase list --since 2026-05-01
nutlog purchase list --product 1
nutlog purchase show 42
nutlog purchase delete 42
```

### Stores

```bash
nutlog store create "Supermercado XYZ"
nutlog store list
nutlog store show 1
nutlog store rename 1 --name "New Name"
nutlog store tag add 1 --tag supermarket
nutlog store delete 1
```

### Store Tags

Same pattern as product tags:

```bash
nutlog store-tag create "supermarket"
nutlog store-tag list
nutlog store-tag search "super"
nutlog store-tag delete 1
```

### Consumption

```bash
nutlog consumption create 1 \
  --quantity 150 \
  --unit g \
  --date today
```

If `--quantity` / `--unit` omitted, falls back to product's reference quantity (if set).

```bash
nutlog --json consumption list --since 2026-05-01
nutlog consumption delete 5
```

### Reports

```bash
# Nutrition summary derived from consumption records + product nutrition data
nutlog --json report nutrition --since 2026-05-01 --until 2026-05-31

# Spending
nutlog --json report spending
nutlog report spending --by store --since 2026-01-01
nutlog --json report spending --by product
```

Reports support human tables and clean JSON.

## LLM Agent Ergonomics

- Modifying commands return `{ "success": true, "id": N, "message": "..." }` (or failure with reason) under `--json`
- Errors for agents are clear and returned in JSON on failure paths
- Consistent field names in lists
- `--json` is the reliable way to consume output

## Non-Goals (current)

- No web / TUI
- No external API imports
- No inventory tracking
- No multi-currency or photos

## Documentation

Detailed documentation lives in the `docs/` directory:

- [docs/index.md](docs/index.md) — entry point with links to all topics
- Installation, getting started, full command reference, nutrition model, reports, agent/JSON usage, data model details, and troubleshooting.

When the package is installed (via PKGBUILD or equivalent), these files are placed under `/usr/share/doc/nutlog/docs/` for offline reading.

The CLI `--help` text remains the authoritative reference for syntax.

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo run -- --db /tmp/test.db --json product list
```

See [AGENTS.md](AGENTS.md) and [CODING_PRACTICES.md](CODING_PRACTICES.md) for contributor guidelines.

## License

TBD (personal tool for now).
