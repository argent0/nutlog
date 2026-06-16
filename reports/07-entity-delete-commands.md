# Development Report: Entity Delete Commands

**Date**: 2026-06-16  
**Project**: nutlog (Rust CLI for food purchase logging, nutrition tracking, and reports)  
**Feature**: Complete `delete` coverage for all data entities  
**Status**: Implemented, documented, and verified. Three new delete commands added; four entities already had delete. All quality gates passed.  
**Binary**: `nutlog` (version 0.1.0)  
**Key stats**: +381 lines across 14 files (implementation + docs + tests + this report); `cargo test` 20/20 (2 unit + 18 integration).

---

## 1. Initial State and Setup

Before this work, delete was only available on four entities:

| Entity       | Delete | Notes |
|--------------|--------|-------|
| `product`    | Yes    | Fails if purchases exist unless `--force` |
| `product-tag`| Yes    | Unconditional; associations cascade |
| `store`      | Yes    | Unconditional; purchases get `store_id` set to NULL |
| `store-tag`  | Yes    | Unconditional; associations cascade |
| `nutrient`   | **No** | — |
| `purchase`   | **No** | — |
| `consumption`| **No** | — |
| `report`     | N/A    | Read-only derived data |

The gap meant agents and users could create purchase and consumption records but had no CLI path to remove mistaken entries without direct SQL. Custom nutrients could not be cleaned up once created.

**Process followed** (per AGENTS.md):

1. Explored existing command patterns in `src/cli.rs` and `src/main.rs`.
2. Reviewed FK constraints in `src/db.rs` to determine safe delete semantics.
3. Added CLI variants, handlers, error type, and integration tests.
4. Updated user-facing documentation (`README.md`, `docs/*`, `spec/02-grok-spec.md`).
5. Ran `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.

---

## 2. Architectural Decisions & Adherence to Principles

### Scope

- **Minimal, targeted addition.** No schema changes, no new entities, no breaking changes to existing delete behavior.
- Follow the established `nutlog <entity> delete <id>` pattern with `--json` support on all mutating commands.
- Mirror existing safety patterns: destructive deletes that affect related data require `--force`.

### Delete semantics by entity

| Entity        | Command                    | Behavior |
|---------------|----------------------------|----------|
| `purchase`    | `purchase delete <id>`     | Unconditional. Leaf record. |
| `consumption` | `consumption delete <id>`  | Unconditional. Leaf record. |
| `nutrient`    | `nutrient delete <id>`     | Fails if `product_micronutrients` references the nutrient unless `--force` (cascades micronutrient rows via FK) |
| `product`     | (unchanged)                | Fails if purchases exist unless `--force` |
| Tags / stores | (unchanged)                | Unconditional |

### Why `nutrient delete` has a `--force` guard

The DB FK on `product_micronutrients.nutrient_id` is `ON DELETE CASCADE`, so a raw SQL delete would silently remove micronutrient entries from products. The CLI guard matches the `product delete` pattern: fail with an actionable error unless the user explicitly opts in with `--force`.

### What was not changed

- `report` — no delete (derived, not stored entities).
- Product, store, and tag delete behavior.
- JSON success/error envelope shapes.
- Database schema or migrations.

No violations of AGENTS.md / CODING_PRACTICES.md rules.

---

## 3. Implementation Highlights

### CLI (`src/cli.rs`)

Added `Delete` variants to three action enums:

```bash
nutlog nutrient delete <id> [--force]
nutlog purchase delete <id>
nutlog consumption delete <id>
```

Each variant includes doc comments for `--help` output.

### Error type (`src/error.rs`)

New variant:

```text
NutrientHasReferences(i64)
→ "nutrient {id} is referenced by product nutrition data; use --force to delete anyway"
```

### Handlers (`src/main.rs`)

- **`handle_nutrient`**: Counts `product_micronutrients` rows for the nutrient ID; exits with JSON error if count > 0 and no `--force`; otherwise `DELETE FROM nutrients WHERE id = ?`.
- **`handle_purchase`**: `DELETE FROM purchases WHERE id = ?`.
- **`handle_consumption`**: `DELETE FROM consumptions WHERE id = ?`.

All three follow the existing pattern: check `affected == 0` for not-found, print success via `print_success_json(Success::ok(...))` or `quiet_print`, exit 1 on failure with `print_error_json`.

---

## 4. Testing & Verification

### Integration tests (`tests/cli.rs`)

| Test | What it verifies |
|------|------------------|
| `purchase_delete_json` | Create + delete purchase; list no longer contains it |
| `consumption_delete_json` | Create + delete consumption; list no longer contains it |
| `nutrient_delete_unreferenced_json` | Custom nutrient deletes without `--force` |
| `nutrient_delete_without_force_fails_when_referenced` | Referenced nutrient fails; `--force` succeeds |
| `delete_without_force_fails_json` | (pre-existing) Product delete safety |

### Quality gates

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test   # 20/20 passing (2 unit + 18 integration)
```

All gates passed cleanly.

---

## 5. Documentation Updates

| File | Changes |
|------|---------|
| `README.md` | Delete examples for nutrient, purchase, consumption, store-tag; updated safety bullet |
| `docs/command-reference.md` | Entities table + three new delete sections + common error |
| `docs/agent-usage.md` | Cleanup workflow and error-handling for new commands |
| `docs/troubleshooting.md` | Nutrient delete failures; stale ID guidance |
| `docs/nutrition-tracking.md` | Corrected data-integrity note (CLI guard, not FK restrict) |
| `docs/getting-started.md` | Short delete examples |
| `docs/data-model.md` | Delete behavior table; FK cascade note on `product_micronutrients` |
| `docs/index.md` | Safety principle mentions nutrient delete |
| `spec/02-grok-spec.md` | Spec examples for new commands |

---

## 6. Deliverables Created / Modified

**Modified (core)**:

- `src/cli.rs` — `Delete` variants on `NutrientAction`, `PurchaseAction`, `ConsumptionAction`
- `src/error.rs` — `NutrientHasReferences`
- `src/main.rs` — delete handlers for nutrient, purchase, consumption
- `tests/cli.rs` — four new integration tests

**Modified (documentation)**:

- `README.md`, `docs/*.md` (8 files), `spec/02-grok-spec.md`

**Created**:

- `reports/07-entity-delete-commands.md` (this report)

---

## 7. Example Usage

```bash
# Remove a mistaken purchase
nutlog --json purchase delete 42

# Remove a consumption logged by error
nutlog --json consumption delete 5

# Remove an unused custom nutrient
nutlog --json nutrient create "Temp Nutrient" --unit mg
nutlog --json nutrient delete 17

# Remove a nutrient still referenced in product nutrition (destructive)
nutlog nutrient delete 17 --force
```

**JSON success**:

```json
{ "success": true, "message": "Deleted purchase 42" }
```

**JSON error** (nutrient referenced):

```json
{ "success": false, "error": "nutrient 17 is referenced by product nutrition data; use --force to delete anyway" }
```

---

## 8. Commands Used for Final Verification

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test

# Targeted tests
cargo test purchase_delete_json
cargo test consumption_delete_json
cargo test nutrient_delete
```

---

**Conclusion**: All data entities now expose a consistent `delete` subcommand. Leaf records (purchase, consumption) delete unconditionally. Referential safety is preserved for `product` and `nutrient` via `--force`. Documentation and tests are aligned with the implementation.

This report is saved at `reports/07-entity-delete-commands.md`.