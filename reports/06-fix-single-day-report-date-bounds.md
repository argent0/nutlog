# Development Report: Fix Single-Day Report Date Bounds

**Date**: 2026-06-16  
**Project**: nutlog (Rust CLI for food purchase logging, nutrition tracking, and reports)  
**Issue**: `nutlog report nutrition --since DATE --until DATE` returned zero items when `since` and `until` were the same calendar day, even when consumptions existed for that day.  
**Status**: Root cause identified and fixed. Single-day ranges now include the full local calendar day. All quality gates passed.  
**Binary**: `nutlog` (version 0.1.0)  
**Key stats**: New `DateBound` enum + `parse_flexible_date_bound()` in `src/db.rs`; 7 `--until` call sites updated in `src/main.rs`; 2 unit tests + 1 integration test added; `cargo test` 16/16.

---

## 1. Initial State and Setup

The bug was documented in `spec/05-single-day-report-bug.md`. Reproduction:

```bash
nutlog --json report nutrition --since 2026-06-16 --until 2026-06-16
```

With consumptions logged at afternoon local times (e.g. `2026-06-16T13:00:00-03:00`), the report returned:

```json
"total_consumed_items": 0
```

Wider ranges (e.g. `--since 2026-06-01 --until 2026-06-30`) included the same data correctly.

**Root cause**: `db::parse_flexible_date` always normalized date-only inputs to **local midnight**. Both `--since` and `--until` therefore resolved to the same UTC instant. The SQL filter became an equality check on a single second:

```sql
AND c.consumed_at >= '2026-06-16T03:00:00Z'
AND c.consumed_at <= '2026-06-16T03:00:00Z'
```

Any consumption after local midnight was excluded.

**Process followed** (per AGENTS.md):

1. Read the bug spec and traced date parsing in `src/db.rs`.
2. Confirmed all `--since` / `--until` filter sites in `src/main.rs` (reports, consumption list, purchase list).
3. Introduced explicit start-of-day / end-of-day semantics without changing create-path behavior.
4. Added unit and integration tests.
5. Ran `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.

---

## 2. Architectural Decisions & Adherence to Principles

### Scope

- **Minimal, targeted fix.** No schema changes, no CLI surface changes, no JSON shape changes.
- `--since` on date-only input → local **00:00:00** (unchanged semantics).
- `--until` on date-only input → local **23:59:59** (new semantics for upper bound).
- Explicit RFC3339 timestamps (with time component) are returned verbatim for both bounds — agents passing precise instants are unaffected.
- `parse_flexible_date()` still resolves to start-of-day for `--date` on create commands (consumption, purchase).

### Why a new function instead of changing `parse_flexible_date`

Create paths (`consumption create --date today`, `purchase create --date today`) intentionally store local midnight for date-only input. Changing the global parser would have broken that contract. A `DateBound` parameter keeps the two use cases explicit:

| Call site | Function | Bound |
|-----------|----------|-------|
| `consumption create --date` | `parse_flexible_date` | Start |
| `purchase create --date` | `parse_flexible_date` | Start |
| `--since` filters | `parse_flexible_date_bound(..., Start)` | Start |
| `--until` filters | `parse_flexible_date_bound(..., End)` | End |

### Affected commands

All commands using `--until` with date-only input:

- `report nutrition`
- `report spending` (total, by-store, by-product queries)
- `consumption list`
- `purchase list`

No violations of AGENTS.md / CODING_PRACTICES.md rules.

---

## 3. Coverage of the Problem

### Before the fix

```text
--since 2026-06-16  →  2026-06-16T03:00:00Z  (local midnight, UTC-3)
--until 2026-06-16  →  2026-06-16T03:00:00Z  (same instant)
```

Single-day report: **0 items**.

### After the fix

```text
--since 2026-06-16  →  2026-06-16T03:00:00Z  (local 00:00:00)
--until 2026-06-16  →  2026-06-17T02:59:59Z  (local 23:59:59)
```

Single-day report: **all consumptions on that local calendar day**.

### What remained unchanged

- JSON report structure (`NutritionReport`, `MacroTotals`, etc.).
- Nutrition aggregation and scaling logic.
- Human-readable report output formatting.
- Create-command date handling (`--date today` still stores local midnight).
- Explicit RFC3339 input handling (returned as-is).

---

## 4. Implementation Highlights

1. **`DateBound` enum** (`src/db.rs`): `Start` and `End` variants document intent at call sites.

2. **`parse_flexible_date_bound(s, bound)`** (`src/db.rs`):
   - Tries RFC3339 on the trimmed original string first (preserves timezone and time).
   - Falls back to flexible date parsing (`today`, `yesterday`, `YYYY-MM-DD`, etc.).
   - Applies `00:00:00` or `23:59:59` in local time, then converts to UTC.

3. **`parse_flexible_date`** now delegates to `parse_flexible_date_bound(s, DateBound::Start)` — single code path, no duplication.

4. **Seven `--until` sites updated** in `src/main.rs` to use `DateBound::End`. `--since` sites continue using start-of-day (via existing `parse_flexible_date` or explicit `DateBound::Start`).

5. **RFC3339 check moved earlier** in the parser (before lowercasing), so explicit timestamps are handled consistently regardless of bound.

---

## 5. Testing & Verification

### Unit tests (`src/db.rs`)

- `date_bound_end_is_after_start_for_same_day` — end bound is later than start for the same `YYYY-MM-DD` input; local time is 23:59:59.
- `explicit_rfc3339_ignores_bound` — RFC3339 with time component returns the same instant for both bounds.

### Integration test (`tests/cli.rs`)

- `report_nutrition_single_day_range_includes_afternoon_consumption` — creates a consumption at 14:30 local time today, runs `report nutrition --since DATE --until DATE` with today's date string, asserts `total_consumed_items: 1` and correct protein total.

### Quality gates

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test   # 16/16 passing (2 unit + 14 integration)
```

All gates passed cleanly.

---

## 6. Deliverables Created / Modified

**Modified (core)**:

- `src/db.rs` — `DateBound` enum, `parse_flexible_date_bound()`, refactored `parse_flexible_date()`, unit tests.
- `src/main.rs` — all `--until` filter sites use `DateBound::End`.
- `tests/cli.rs` — single-day range integration test.
- `Cargo.toml` — `chrono` added to dev-dependencies for the integration test.

**Created**:

- `reports/06-fix-single-day-report-date-bounds.md` (this report).
- `spec/05-single-day-report-bug.md` (original bug report, used as spec input).

---

## 7. Current State & Next Steps (Optional)

Single-day `report nutrition`, `report spending`, `consumption list`, and `purchase list` now behave as users expect when `--since` and `--until` are the same date.

**Future polish ideas** (low priority):

- Document `DateBound` semantics in `docs/agent-usage.md` (currently describes only start-of-day).
- Consider a `--day DATE` sugar flag on report commands that expands to the correct 24-hour range internally.
- Sub-second precision on `--until` (e.g. `23:59:59.999`) if storage format ever moves beyond second resolution.

---

## 8. Commands Used for Final Verification

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test

# Reproduce the reported scenario (after fix)
cargo run -- --json report nutrition --since 2026-06-16 --until 2026-06-16

# Targeted tests
cargo test date_bound
cargo test report_nutrition_single_day_range_includes_afternoon_consumption -- --nocapture
```

---

**Conclusion**: The bug was caused by both range bounds resolving to the same instant for date-only input. The fix introduces explicit start-of-day and end-of-day semantics for filter bounds while preserving existing create-path behavior. Daily nutrition tracking with `today` or `YYYY-MM-DD` date ranges now works correctly.

This report is saved at `reports/06-fix-single-day-report-date-bounds.md`.