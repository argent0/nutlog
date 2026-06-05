# nutlog

`nutlog` is a simple, local, CLI-first tool to log food purchases, track nutrition, and generate basic reports. It is designed primarily to be used by LLM agents on behalf of a single user.

## Technical Details

- **Language**: Rust
- **Interface**: Command Line Interface (CLI)
- **Output formats**: Human-readable (default) and structured JSON (`--json` flag)
- **Database**: SQLite (embedded)
- **Binary name**: `nutlog`
- **Money storage**: All monetary values are stored as **integers in cents** (e.g. `1999` = $19.99)
- **Timestamps**: Stored in UTC. Human output converts to local timezone. JSON output includes both UTC and local time when relevant.
- **Installation**: Via the user's Linux distribution package manager (or `cargo install` during development)

## Objectives & Design Principles

- **Linux-only**, CLI forever.
- **Single-user** only. No authentication, no multi-tenancy.
- **LLM-agent first**: The tool must be predictable, have consistent command structure, excellent `--json` output, clear error messages, and machine-readable help where possible.
- **Simplicity over features**: Prefer a small, coherent set of commands over a large incomplete feature set.
- **Privacy & offline-first**: All data stays on the user's machine.
- **Data quality**: Make it easy to record purchases and nutrition without forcing perfect data entry on every use.

## Global Flags

```bash
nutlog [GLOBAL OPTIONS] <ENTITY> <ACTION> [ARGS]
```

Supported global flags:

| Flag          | Description                                      | Default          |
|---------------|--------------------------------------------------|------------------|
| `--json`      | Output structured JSON instead of human text     | false            |
| `--db <PATH>` | Override default SQLite database location        | XDG data dir     |
| `--quiet`     | Minimal output (useful for scripting/LLMs)       | false            |
| `--help`      | Show help                                        | -                |
| `--version`   | Show version                                     | -                |

## Command Structure (Consistent Pattern)

All commands follow the pattern:

```
nutlog <entity> <action> [options]
```

Main entities:
- `product`
- `nutrient`
- `product-tag`
- `purchase`
- `store`
- `store-tag`
- `consumption`
- `report`

---

## Data Model Overview (High Level)

- **Product**: A specific food item the user buys (e.g. "YOUGURISIMO 300G NATU"). Can have tags and nutritional information.
- **Nutrient**: Master list of nutrients (protein, vitamin C, etc.). Contains both common pre-populated nutrients and user-defined ones.
- **Product Tag** & **Store Tag**: Simple taxonomies.
- **Purchase**: Record of buying a product (with optional price, store, date, and quantity).
- **Consumption**: Record of actually eating/drinking a product (supports partial consumption).
- **Store**: Optional CRM for where purchases happen.

**Relationship note**: A purchase increases what the user has bought. A consumption records what was actually eaten. The tool does **not** assume 1:1 equivalence between purchases and consumption.

---

## Products

### Create a product

```bash
nutlog product create "YOUGURISIMO 300G NATU" --tags yogurt,natural
nutlog product create "Banana" --tags fruit
```

### List products

```bash
nutlog product list
nutlog --json product list
```

### Search products

```bash
nutlog --json product search --name "yogu"
nutlog --json product search --tag yogurt
```

Performs fuzzy search on name and ranks results by relevance.

### Show product details

```bash
nutlog product show <product-id>
nutlog --json product show <product-id>
```

### Rename a product

```bash
nutlog product rename <product-id> --name "New Name"
```

### Add/remove tags from a product

```bash
nutlog product tag add <product-id> --tag yogurt
nutlog product tag remove <product-id> --tag yogurt
```

### Delete a product (with safety)

```bash
nutlog product delete <product-id>
```

- Fails if the product has associated purchases (unless `--force` is used).
- LLM-friendly: returns clear reason in JSON.

---

## Nutritional Information

Products can have nutritional data attached. The model uses a clear **reference amount**.

### Recommended JSON structure for `nutritional_information`

```json
{
  "reference": {
    "quantity": 100,
    "unit": "g"
  },
  "energy_kcal": 123,
  "protein_g": 8.5,
  "carbohydrates_g": 12.3,
  "fat_g": 3.2,
  "fiber_g": 1.1,
  "sugars_g": 4.8,
  "micronutrients": [
    {
      "nutrient_id": 42,
      "amount": 0.45,
      "unit": "mg"
    }
  ]
}
```

### Attach or update nutrition for a product

```bash
nutlog product nutrition set <product-id> \
  --reference-quantity 100 \
  --reference-unit g \
  --energy-kcal 123 \
  --protein-g 8.5 \
  --carbohydrates-g 12.3
```

### View nutrition

```bash
nutlog --json product show <product-id>   # includes nutritional_information
```

---

## Nutrients (Master List / CRM)

```bash
nutlog nutrient list
nutlog --json nutrient list
nutlog nutrient create "Vitamin D" --unit µg --recommended-intake 15
nutlog nutrient show <nutrient-id>
nutlog nutrient search "vit d"
```

- A reasonable set of common nutrients should be pre-populated on first run (or via migration).
- Users can add custom nutrients.
- `recommended_intake` is optional and stored as a simple value + unit.

---

## Product Tags

```bash
nutlog product-tag create "yogurt"
nutlog product-tag list
nutlog --json product-tag search "yo"
nutlog product-tag show <tag-id>
nutlog product-tag delete <tag-id>
```

---

## Purchases

### Create a purchase

```bash
nutlog purchase create <product-id> \
  --price 19.99 \
  --store <store-id> \
  --date yesterday \
  --quantity 2
```

- `--quantity` defaults to `1`
- `--price` can be given with or without currency symbol (internally converted to cents)
- `--date` supports natural language: `today`, `yesterday`, `2026-05-20`, `last week`, etc.
- Price and store are optional.

### List & search purchases

```bash
nutlog purchase list --since 2026-05-01 --json
nutlog purchase list --product <product-id>
nutlog purchase list --store <store-id>
```

### Show a purchase

```bash
nutlog purchase show <purchase-id>
```

---

## Stores (Light CRM)

```bash
nutlog store create "Supermercado XYZ"
nutlog store list
nutlog store show <store-id>
nutlog store rename <store-id> --name "New Name"
nutlog store tag add <store-id> --tag supermarket
nutlog store delete <store-id>
```

---

## Store Tags

Same pattern as product tags:

```bash
nutlog store-tag create "supermarket"
nutlog store-tag list
nutlog store-tag search "super"
```

---

## Consumption

Records what was actually eaten (supports partial consumption).

```bash
nutlog consumption create <product-id> \
  --quantity 150 \
  --unit g \
  --date today
```

- If `--quantity` / `--unit` are omitted, the tool may suggest using the product's reference quantity.
- Useful for accurate nutrition tracking when the user doesn't eat the entire purchased item.

### List consumption

```bash
nutlog consumption list --since 2026-05-01 --json
```

---

## Reports (High-Value Addition)

```bash
# Nutrition summary
nutlog report nutrition --since 2026-05-01 --until 2026-05-31 --json

# Spending summary
nutlog report spending --period month --json
nutlog report spending --by store --since 2026-01-01
```

Reports should support both human-readable tables and clean JSON.

---

## LLM Agent Ergonomics (Important)

- Every modifying command (`create`, `delete`, `rename`, etc.) must return a clear success/failure object when `--json` is used.
- All list/search commands should be stable and return consistent field names.
- Consider adding a machine-readable command reference in the future:

```bash
nutlog schema commands --json
```

- Error messages should be actionable and non-verbose by default (use `--verbose` for more details).

---

## Database

- **Location**: Follows XDG Base Directory Specification by default (`$XDG_DATA_HOME/nutlog/nutlog.db` or `~/.local/share/nutlog/nutlog.db`).
- Override with `--db /path/to/file.db`
- Uses proper migrations (schema versioning).
- All timestamps stored as UTC (`TEXT` or `INTEGER` unix timestamp — decision left to implementation).

---

## Non-Goals (for now)

- No web interface or TUI
- No automatic import from external APIs (user can add data manually or via LLM)
- No inventory tracking / "running out" alerts
- No multi-currency support
- No photo upload or barcode scanning
