# Bug Report: `nutlog report nutrition` returns 0 items for single-day ranges

## Summary
When running `nutlog report nutrition --since 2026-06-16 --until 2026-06-16`, the command returns `total_consumed_items: 0` even though valid consumption records exist for that calendar day. The same data is correctly included when using a wider date range.

## Steps to Reproduce
1. Log several consumptions on 2026-06-16 with explicit local timestamps (e.g. `2026-06-16T13:00:00-03:00`, `2026-06-16T16:00:00-03:00`, etc.).
2. Run:
   ```bash
   nutlog --json report nutrition --since 2026-06-16 --until 2026-06-16
   ```
3. Observe `total_consumed_items: 0` and all totals are `null`.

## Expected Behavior
A single-day report should return all consumption records whose `consumed_at` falls within the logical calendar day (local time), with correctly scaled nutrition totals.

## Actual Behavior
The report returns zero items because the generated SQL `WHERE` clause becomes an equality filter on a single instant.

## Root Cause
**File:** `rust/pkg/nutlog/src/nutlog/src/db.rs`, function `parse_flexible_date` (lines 205–258)

**Location in report logic:** `main.rs:1579–1591` (nutrition report) and similar code for spending reports.

The function always normalizes both `since` and `until` dates to **local midnight**:

```rust
let local_dt = naive
    .and_hms_opt(0, 0, 0)                    // ← hardcoded start of day
    .and_local_timezone(chrono::Local)
    ...
Ok(local_dt.with_timezone(&Utc))
```

When the CLI builds the query:

```rust
// since
AND c.consumed_at >= '2026-06-16T03:00:00Z'
// until
AND c.consumed_at <= '2026-06-16T03:00:00Z'
```

Both bounds resolve to the exact same timestamp (`2026-06-16T03:00:00Z`). The query therefore only matches records with a timestamp at that precise second.

All real consumption records for 2026-06-16 have later local times (13:00, 16:00, 18:11, 21:11, etc.) and are excluded.

Wider ranges work accidentally because `--until 2026-06-16` happens to be interpreted as the start of the next day in some contexts.

## Environment
- **OS:** Linux (Arch)
- **nutlog binary:** `/usr/bin/nutlog` (stripped)
- **Source:** `/home/aner/rust/pkg/nutlog/src/nutlog/`
- **Timezone:** America/Argentina/Buenos_Aires (UTC-3)
- **Database:** `~/.local/share/nutlog/nutlog.db`
- **Date handling library:** `chrono`

## Suggested Fix
Modify date handling so that:
- `--since` resolves to **start of day** (local 00:00)
- `--until` resolves to **end of day** (local 23:59:59.999)

Alternatively, accept an optional time component or add a `--day` / `--date` flag that internally expands to a proper 24-hour range.

## Severity
Medium — breaks a core reporting workflow for daily nutrition tracking, especially when users follow the documented pattern of using `today` or explicit `YYYY-MM-DD` dates.