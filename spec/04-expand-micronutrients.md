# Feature Request: Better Support for Dietary Supplements & Micronutrients

**Date**: 2026-06-06  
**Submitted by**: Aner (via Hermes agent)  
**Related products logged**: Creatine Monohydrate, Magnesium (Trimagnesio), Hydrolyzed Collagen + Hyaluronic Acid + Vitamin C, Omega-3 Fish Oil (EPA/DHA)

## Current Limitations

While `nutlog` already has a solid data model for micronutrients (`product_micronutrients` table + master `nutrients` table), the **CLI experience** makes it impractical to track real dietary supplements effectively:

### 1. CLI `product nutrition set` only supports basic macros
- Only `--energy-kcal`, `--protein-g`, `--carbohydrates-g`, `--fat-g`, `--fiber-g`, `--sugars-g` are available.
- No way to set micronutrients or active compounds through the CLI.
- Micronutrients require manual SQL insertion, which is not agent-friendly or sustainable.

### 2. No support for supplement-specific active ingredients
Common supplement components have no natural home:
- Creatine monohydrate (g)
- Elemental magnesium (mg)
- EPA / DHA (mg)
- Hydrolyzed collagen (g)
- Hyaluronic acid (mg)
- etc.

These must be crammed into the product name, losing structured data and reporting capability.

### 3. Reports cannot surface what matters for supplements
Even though the schema supports scaling micronutrients, because there's no easy input path, reports only show energy + macros. Users tracking supplements care primarily about:
- Total creatine ingested this week
- Average daily magnesium
- EPA + DHA totals
- Collagen intake over time

### 4. Pre-populated nutrients are insufficient for supplements
Current list (Protein, Carbs, Fat, Fiber, Sugars, Vitamin C/D, Calcium, Iron, Potassium) does not cover the most common supplement actives.

## Proposed Solution

### Phase 1 (High Impact, Low Effort)
Extend the existing `product nutrition set` command:

```bash
nutlog product nutrition set 13 \
  --reference-quantity 1 --reference-unit capsule \
  --energy-kcal 10 \
  --micronutrient "Omega 3 EPA" 181 mg \
  --micronutrient "Omega 3 DHA" 121 mg \
  --micronutrient "Creatine Monohydrate" 5 g
```

Or support a JSON nutrition payload for complex cases:

```bash
nutlog product nutrition set 13 --json-file nutrition.json
```

### Phase 2
- Add common supplement nutrients to the default seed list (Creatine, EPA, DHA, Magnesium elemental, Collagen peptides, etc.).
- Add a `nutlog supplement` or `nutlog active` helper command for common patterns.
- Improve `report nutrition` to optionally show micronutrient totals with units.

### Benefits
- Makes `nutlog` genuinely useful for the large category of users taking sports nutrition / dietary supplements.
- Leverages the existing excellent schema instead of working around it.
- Keeps the simple macro path fast while unlocking the full power of the data model for advanced use cases.

## References
- `docs/nutrition-tracking.md` explicitly acknowledges this gap ("Micronutrients must currently be inserted manually").
- Current products affected: IDs 10 (Creatine), 11 (Magnesium), 12 (Collagen), 13 (Fish Oil).

This feature would significantly increase the practical value of nutlog for supplement tracking without changing its core simple design.