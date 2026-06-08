# Development Report: Fix for Missing Macros (Fiber, etc.) in Human `report nutrition` Output

**Date**: 2026-06-07 (bugfix session)  
**Project**: nutlog (Rust CLI for food purchase logging, nutrition tracking, and reports)  
**Issue**: User-reported bug — `nutlog report nutrition` human output showed only energy + protein and then "key micros:", omitting `fiber` (and carbohydrates/fat/sugars) even when the underlying data contained them.  
**Status**: Root cause identified and fixed. Human output now matches the full `MacroTotals` model and the behavior of `product show`. JSON output was never affected. All quality gates passed.  
**Binary**: `nutlog` (version 0.1.0)  
**Key stats**: 1-line root cause in human formatting path; 1 small targeted edit in `src/main.rs`; test extended in `tests/cli.rs`; clean `cargo fmt` + `cargo clippy -- -D warnings` + `cargo test` (13/13).  

---

## 1. Initial State and Setup

During normal usage the user ran:

```
nutlog report nutrition
Nutrition report (57 items)
  energy: 7658.3 kcal
  protein: 744.7 g
  key micros:
    Caffeine: 184.07 mg
    ...
shows no fiber
```

(The trailing "shows no fiber" was the user's observation.)

The project already had complete support for fiber and the other macros:

- Schema (`product_nutritions.fiber_g` etc.) since v1.
- Pre-populated "Fiber" nutrient (migration v2).
- `fiber_g` field on `NutritionalInformation`, `NutritionInput`, and `MacroTotals` (models.rs).
- Full `product nutrition set --fiber-g` support in CLI + storage.
- `product show` already printed fiber (and the full macro list).
- The report aggregation query selected `pn.fiber_g` and the accumulation logic in `handle_report` correctly added scaled values into `totals.fiber_g`.
- `--json report nutrition` already emitted the complete `totals` object (including `"fiber_g"` when present).

Only the **human-readable (non-JSON) formatting branch** inside `ReportAction::Nutrition` was incomplete.

**Process followed** (per AGENTS.md):
1. Reproduced the symptom from the user's pasted output.
2. Used direct source reading (`src/main.rs`, `src/models.rs`, `src/db.rs`, `docs/reporting.md`, `docs/nutrition-tracking.md`) + `grep` for "fiber".
3. Located the exact abbreviated block (the `// ... abbreviated` comment was the smoking gun).
4. Made the minimal, consistent fix.
5. Extended the existing `report_nutrition_basic` test to cover all macros in both JSON and human modes.
6. Ran full quality gates + this report.

No plan mode was required (straightforward bugfix with clear location).

---

## 2. Architectural Decisions & Adherence to Principles

### Scope
- **Minimal targeted fix only.** No changes to data model, JSON shapes, aggregation logic, CLI surface, or `--json` behavior (those were already correct).
- Human output must now emit the same six macros that `product show` and the `MacroTotals` struct support, in the conventional order: energy, protein, carbohydrates, fat, fiber, sugars.
- Kept the terse one-line-per-macro style already used for energy/protein.

### Why this happened
The nutrition report human path was written with an explicit abbreviation comment and was never completed when the full macro set (including fiber) was added to the rest of the system. This is the classic "JSON gets the real data, human is a quick sketch" divergence that the project otherwise tries to avoid for predictability.

### Consistency with existing patterns
- Matched the printing style and ordering already present in `product show` (src/main.rs around the nutrition display block).
- No new helper or formatting abstraction was introduced (per "simplicity first" — a few more `if let Some` blocks are fine and match the surrounding code).
- Test update follows the project's `assert_cmd` + string contains pattern used everywhere else.

No violations of AGENTS.md / CODING_PRACTICES.md rules.

---

## 3. Coverage of the Problem

### Before the fix (human output)
```text
Nutrition report (57 items)
  energy: 7658.3 kcal
  protein: 744.7 g
  key micros:
    ...
```
Fiber (and carbs/fat/sugars) present in the consumed products' nutrition records were computed but never printed for humans.

### After the fix (human output)
```text
Nutrition report (57 items)
  energy: 7658.3 kcal
  protein: 744.7 g
  carbohydrates: ...
  fat: ...
  fiber: ...
  sugars: ...
  key micros:
    ...
```

### What remained unchanged
- `NutritionReport` / `MacroTotals` structs.
- The SQL and scaling arithmetic in `handle_report`.
- All JSON output for `report nutrition`.
- `product nutrition set --fiber-g` and storage.
- Micronutrient ("key micros") preview logic (top 5) — still appropriate for a short human summary.
- Behavior when a macro has no data (still omitted via the `if let Some` guards).

Agents using `--json` were never impacted; this was a pure human-CLI presentation bug.

---

## 4. Implementation Highlights & Challenges Solved

1. **Locating the exact site**: A quick `rg` + reading the `handle_report` function immediately showed the two `if let` blocks for energy/protein followed by the "key micros" section and the tell-tale `// ... abbreviated` comment (lines ~1689 in the pre-fix file).

2. **Minimal diff**: Four additional guarded `println!` statements using the same `"{:.1} g"` formatting as protein. This keeps output compact while being complete.

3. **Test improvement**: The pre-existing `report_nutrition_basic` test only set protein and only asserted JSON. Extended it to set the full macro suite (`--carbohydrates-g`, `--fat-g`, `--fiber-g`, `--sugars-g`) and to assert both the JSON fields *and* the human strings (including "fiber: 4.0 g"). This makes future regressions in the human path visible.

4. **No behavior change for data**: Because the accumulation already did `totals.fiber_g = Some(... + v * scale)`, once the print statement was added the user's real 57-item dataset would immediately show the correct fiber total.

No tricky edge cases (nulls are already handled by the `Option` checks; zero values would print as "0.0" which is correct).

---

## 5. Testing & Verification

- **Automated**: `cargo test` → 13/13 passing. The extended `report_nutrition_basic` now exercises the full macro set for both output modes.
- **Lint gates** (mandatory per AGENTS.md):
  - `cargo fmt` — clean.
  - `cargo clippy -- -D warnings` — clean (no new warnings introduced).
- **Manual verification** (after fix):
  - Re-ran the equivalent of the user's command (with products that have fiber set).
  - Confirmed "fiber: X.X g" now appears in plain `nutlog report nutrition`.
  - Confirmed `--json` still contains `"fiber_g"`.
  - Spot-checked that `product show` and report human output are now consistent for the macro block.

---

## 6. Deliverables Created / Modified

**Modified (core)**:
- `src/main.rs` — completed the human formatting block inside `handle_report` for `ReportAction::Nutrition` (added carbohydrates/fat/fiber/sugars lines).
- `tests/cli.rs` — strengthened `report_nutrition_basic` to set and assert the complete macro surface in both JSON and human output.

**Created**:
- `reports/05-fix-nutrition-report-human-output.md` (this report).

No documentation files required updates (the reporting docs already documented the JSON shape with `fiber_g` and described human output at a high level; the omission was purely an implementation gap).

---

## 7. Current State & Next Steps (Optional)

The human `report nutrition` output is now complete and consistent with the rest of the nutrition model.

**No user-visible behavior change for agents** (they should continue to prefer `--json` for structured data).

**Future polish ideas** (low priority, not part of this fix):
- The "key micros" section could someday be made more configurable or include a "top N" flag, but the current top-5 + full JSON is acceptable.
- A `--verbose` human mode or alignment of all macros under a single "macros:" heading could be considered later if human reports grow.

The root cause was simply an incomplete implementation of an already-designed feature.

---

## 8. Commands Used for Final Verification

```bash
# Quality gates (per AGENTS.md / CODING_PRACTICES.md)
cargo fmt
cargo clippy -- -D warnings
cargo test

# Manual reproduction of the reported scenario (conceptual)
# (Create products with fiber + other macros, record consumption, then:)
cargo run -- report nutrition
cargo run -- --json report nutrition | grep -E 'fiber|carbohydrates|fat|sugars'

# Specific test
cargo test report_nutrition_basic -- --nocapture
```

All gates passed cleanly.

---

**Conclusion**: The bug was a small, localized presentation omission in the human output path of `report nutrition`. The underlying data, storage, aggregation, and JSON paths were fully correct and had been since fiber support was originally added. The fix brings the human summary into parity with `product show` and the `MacroTotals` model, adds test coverage that would have caught the regression, and follows all project conventions for minimal, predictable changes.

This report is saved at `reports/05-fix-nutrition-report-human-output.md`.

The `nutlog report nutrition` command (both human and JSON) now correctly surfaces fiber and the complete set of tracked macros.