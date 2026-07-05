mod cli;
mod db;
mod error;
mod models;

use anyhow::Result;
use clap::Parser;
use rusqlite::Connection;

use cli::{Cli, Commands};
use db::open_db;

fn main() {
    if let Err(e) = run() {
        // For agent friendliness, always exit non-zero on error.
        // Human: print to stderr, JSON: ? For now, if json context hard, just print error.
        // Simple: always human error to stderr for now; commands handle their json errors.
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let mut conn = open_db(cli.db.as_deref())?;

    match cli.command {
        Commands::Product { action } => {
            commands::handle_product(&mut conn, action, cli.json, cli.quiet)?;
        }
        Commands::Nutrient { action } => {
            commands::handle_nutrient(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::ProductTag { action } => {
            commands::handle_product_tag(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::Purchase { action } => {
            commands::handle_purchase(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::Store { action } => {
            commands::handle_store(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::StoreTag { action } => {
            commands::handle_store_tag(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::Consumption { action } => {
            commands::handle_consumption(&conn, action, cli.json, cli.quiet)?;
        }
        Commands::Report { action } => {
            commands::handle_report(&conn, action, cli.json, cli.quiet)?;
        }
    }

    Ok(())
}

// Bring in commands module (defined below in this file for simplicity, or split later)
mod commands {
    use super::*;
    use crate::cli::*;
    use crate::db::{
        format_local, local_date_from_rfc3339, now_utc, parse_flexible_date,
        parse_flexible_date_bound, resolve_nutrition_period, DateBound, ResolvedNutritionPeriod,
    };
    use crate::error::{NutlogError, Result as NutResult};
    use crate::models::*;
    use comfy_table::{presets, Cell, Table};
    use rusqlite::{params, OptionalExtension, Row};
    use std::collections::{BTreeMap, HashMap};
    use strsim::jaro_winkler;

    // ---------- helpers ----------

    fn print_json<T: serde::Serialize>(v: &T) {
        println!("{}", serde_json::to_string_pretty(v).unwrap());
    }

    fn print_success_json(success: Success) {
        print_json(&success);
    }

    fn print_error_json(err: &str) {
        #[derive(serde::Serialize)]
        struct ErrOut {
            success: bool,
            error: String,
        }
        print_json(&ErrOut {
            success: false,
            error: err.to_string(),
        });
    }

    fn quiet_print(msg: &str, quiet: bool) {
        if !quiet {
            println!("{}", msg);
        }
    }

    fn cents_to_str(cents: i64) -> String {
        let sign = if cents < 0 { "-" } else { "" };
        let abs = cents.abs();
        format!("{}{}.{:02}", sign, abs / 100, abs % 100)
    }

    fn parse_price_to_cents(s: &str) -> NutResult<i64> {
        let s = s.trim();
        let s = s.strip_prefix('$').unwrap_or(s).trim();
        let val: f64 = s
            .parse()
            .map_err(|_| NutlogError::InvalidPrice(s.to_string()))?;
        if !val.is_finite() || val < 0.0 {
            return Err(NutlogError::InvalidPrice(s.to_string()));
        }
        Ok((val * 100.0).round() as i64)
    }

    fn format_price_opt(cents: Option<i64>) -> Option<String> {
        cents.map(|c| format!("${}", cents_to_str(c)))
    }

    fn row_to_product(conn: &Connection, row: &Row) -> rusqlite::Result<Product> {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let created: String = row.get(2)?;
        let updated: String = row.get(3)?;

        // tags
        let mut stmt = conn.prepare(
            "SELECT pt.name FROM product_tags pt
             JOIN product_tag_associations pta ON pta.tag_id = pt.id
             WHERE pta.product_id = ? ORDER BY pt.name",
        )?;
        let tags: Vec<String> = stmt
            .query_map([id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // nutrition
        let nutrition = load_nutrition(conn, id)?;

        Ok(Product {
            id,
            name,
            tags,
            nutritional_information: nutrition,
            created_at: crate::db::make_timestamp_info(&created),
            updated_at: crate::db::make_timestamp_info(&updated),
        })
    }

    fn load_nutrition(
        conn: &Connection,
        product_id: i64,
    ) -> rusqlite::Result<Option<NutritionalInformation>> {
        #[allow(clippy::type_complexity)]
        let base: Option<(f64, String, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>)> = conn.query_row(
            "SELECT reference_quantity, reference_unit, energy_kcal, protein_g, carbohydrates_g, fat_g, fiber_g, sugars_g
             FROM product_nutritions WHERE product_id = ?",
            [product_id],
            |r| Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?
            )),
        ).optional()?;

        match base {
            None => Ok(None),
            Some((qty, unit, e, p, c, f, fi, su)) => {
                let mut micros = vec![];
                let mut stmt = conn.prepare(
                    "SELECT pm.nutrient_id, pm.amount, pm.unit, n.name
                     FROM product_micronutrients pm
                     JOIN nutrients n ON n.id = pm.nutrient_id
                     WHERE pm.product_id = ? ORDER BY n.name",
                )?;
                let rows = stmt.query_map([product_id], |r| {
                    Ok(Micronutrient {
                        nutrient_id: r.get(0)?,
                        amount: r.get(1)?,
                        unit: r.get(2)?,
                        name: r.get(3)?,
                    })
                })?;
                for m in rows {
                    micros.push(m?);
                }
                Ok(Some(NutritionalInformation {
                    reference: ReferenceAmount {
                        quantity: qty,
                        unit,
                    },
                    energy_kcal: e,
                    protein_g: p,
                    carbohydrates_g: c,
                    fat_g: f,
                    fiber_g: fi,
                    sugars_g: su,
                    micronutrients: micros,
                }))
            }
        }
    }

    fn tokenize(s: &str) -> Vec<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    /// Score how well a single name token matches a single query token.
    fn word_match_score(word: &str, query_word: &str) -> f64 {
        if word == query_word {
            return 1.0;
        }
        if word.starts_with(query_word) {
            return 0.9;
        }
        if query_word.starts_with(word) && word.len() >= 2 {
            return 0.85;
        }
        if query_word.len() >= 2 && word.contains(query_word) {
            return 0.8;
        }
        let len_ratio = word.len() as f64 / query_word.len() as f64;
        if (0.5..=2.0).contains(&len_ratio) {
            let jw = jaro_winkler(word, query_word);
            if jw >= 0.85 {
                return jw;
            }
        }
        0.0
    }

    /// Score how well `name` matches `query` using token-aware matching.
    ///
    /// Full-string Jaro-Winkler is poor for short queries against long product names
    /// (e.g. "milk" ranking "Milanesa" above "whole milk"). We match per-token instead.
    fn name_match_score(name: &str, query: &str) -> f64 {
        let name_lower = name.to_lowercase();
        let query_lower = query.to_lowercase();

        if name_lower == query_lower {
            return 1.0;
        }
        if name_lower.contains(&query_lower) {
            return 0.95;
        }

        let name_words = tokenize(name);
        let query_words = tokenize(query);
        if query_words.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        let mut matched = 0u32;
        for qw in &query_words {
            let best = name_words
                .iter()
                .map(|w| word_match_score(w, qw))
                .fold(0.0f64, f64::max);
            if best > 0.0 {
                matched += 1;
                total += best;
            }
        }

        if matched != query_words.len() as u32 {
            return 0.0;
        }

        total / query_words.len() as f64
    }

    fn fuzzy_rank(items: Vec<(i64, String)>, query: &str) -> Vec<(i64, String, f64)> {
        const MIN_SCORE: f64 = 0.5;
        let mut scored: Vec<_> = items
            .into_iter()
            .map(|(id, name)| {
                let score = name_match_score(&name, query);
                (id, name, score)
            })
            .filter(|(_, _, score)| *score >= MIN_SCORE)
            .collect();
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    // ---------- PRODUCT HANDLERS ----------

    pub fn handle_product(
        conn: &mut Connection,
        action: ProductAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            ProductAction::Create { name, tags } => {
                let now = now_utc();
                conn.execute(
                    "INSERT INTO products (name, created_at, updated_at) VALUES (?1, ?2, ?2)",
                    params![name, now],
                )?;
                let id = conn.last_insert_rowid();

                if let Some(ts) = tags {
                    for t in ts {
                        ensure_product_tag(conn, &t, &now)?;
                        let tag_id: i64 = conn.query_row(
                            "SELECT id FROM product_tags WHERE name = ?1",
                            [&t],
                            |r| r.get(0),
                        )?;
                        let _ = conn.execute(
                            "INSERT OR IGNORE INTO product_tag_associations (product_id, tag_id) VALUES (?1, ?2)",
                            params![id, tag_id],
                        );
                    }
                }

                let msg = format!("Created product {} ({})", id, name);
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            ProductAction::List => {
                let mut stmt = conn.prepare(
                    "SELECT id, name, created_at, updated_at FROM products ORDER BY id DESC",
                )?;
                let rows = stmt.query_map([], |r| row_to_product(conn, r))?;
                let products: Vec<Product> = rows.filter_map(|r| r.ok()).collect();

                if json {
                    print_json(&products);
                } else if quiet {
                    for p in &products {
                        println!("{}: {}", p.id, p.name);
                    }
                } else {
                    let mut table = Table::new();
                    table.load_preset(presets::UTF8_FULL_CONDENSED);
                    table.set_header(vec!["ID", "Name", "Tags"]);
                    for p in &products {
                        table.add_row(vec![
                            Cell::new(p.id),
                            Cell::new(&p.name),
                            Cell::new(p.tags.join(", ")),
                        ]);
                    }
                    println!("{}", table);
                    if products.is_empty() {
                        println!("(no products)");
                    }
                }
            }
            ProductAction::Search { name, tag } => {
                let mut results: Vec<Product> = vec![];

                if let Some(nq) = name {
                    // get candidates
                    let mut stmt = conn.prepare("SELECT id, name FROM products")?;
                    let cands: Vec<(i64, String)> = stmt
                        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .filter_map(|x| x.ok())
                        .collect();
                    let ranked = fuzzy_rank(cands, &nq);
                    for (id, _name, _score) in ranked.into_iter().take(50) {
                        if let Ok(p) = load_full_product(conn, id) {
                            results.push(p);
                        }
                    }
                } else if let Some(tq) = tag {
                    // exact tag filter, but support fuzzy tag? per spec --tag exact? but search by tag.
                    // For now exact match on tag name.
                    let mut stmt = conn.prepare(
                        "SELECT p.id, p.name, p.created_at, p.updated_at
                         FROM products p
                         JOIN product_tag_associations pta ON pta.product_id = p.id
                         JOIN product_tags pt ON pt.id = pta.tag_id
                         WHERE pt.name = ?1 ORDER BY p.id DESC",
                    )?;
                    let rows = stmt.query_map([&tq], |r| row_to_product(conn, r))?;
                    results = rows.filter_map(|r| r.ok()).collect();
                } else {
                    // no filter -> all
                    let mut stmt = conn.prepare(
                        "SELECT id, name, created_at, updated_at FROM products ORDER BY id DESC",
                    )?;
                    let rows = stmt.query_map([], |r| row_to_product(conn, r))?;
                    results = rows.filter_map(|r| r.ok()).collect();
                }

                if json {
                    print_json(&results);
                } else {
                    // simple list
                    for p in &results {
                        println!("{}: {}  [{}]", p.id, p.name, p.tags.join(","));
                    }
                    if results.is_empty() {
                        println!("(no matches)");
                    }
                }
            }
            ProductAction::Show { id } => {
                let p =
                    load_full_product(conn, id).map_err(|_| NutlogError::ProductNotFound(id))?;
                if json {
                    print_json(&p);
                } else {
                    println!("Product {}: {}", p.id, p.name);
                    let tags_str = if p.tags.is_empty() {
                        "(none)".to_string()
                    } else {
                        p.tags.join(", ")
                    };
                    println!("Tags: {}", tags_str);
                    if let Some(nut) = &p.nutritional_information {
                        println!(
                            "Nutrition (per {} {}):",
                            nut.reference.quantity, nut.reference.unit
                        );
                        if let Some(v) = nut.energy_kcal {
                            println!("  energy: {} kcal", v);
                        }
                        if let Some(v) = nut.protein_g {
                            println!("  protein: {} g", v);
                        }
                        if let Some(v) = nut.carbohydrates_g {
                            println!("  carbohydrates: {} g", v);
                        }
                        if let Some(v) = nut.fat_g {
                            println!("  fat: {} g", v);
                        }
                        if let Some(v) = nut.fiber_g {
                            println!("  fiber: {} g", v);
                        }
                        if let Some(v) = nut.sugars_g {
                            println!("  sugars: {} g", v);
                        }
                        if !nut.micronutrients.is_empty() {
                            println!("  micronutrients:");
                            for m in &nut.micronutrients {
                                println!(
                                    "    - {}: {} {} (nutrient #{})",
                                    m.name.as_deref().unwrap_or(""),
                                    m.amount,
                                    m.unit,
                                    m.nutrient_id
                                );
                            }
                        }
                    } else {
                        println!("Nutrition: (none set)");
                    }
                    println!("Created: {}", format_local(&p.created_at.utc));
                }
            }
            ProductAction::Rename { id, name } => {
                let affected = conn.execute(
                    "UPDATE products SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    params![name, now_utc(), id],
                )?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("product not found: {}", id));
                    } else {
                        eprintln!("No such product");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Renamed product {} to {}", id, name);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            ProductAction::Tag { action } => match action {
                ProductTagModifyAction::Add { id, tag } => {
                    let now = now_utc();
                    ensure_product_tag(conn, &tag, &now)?;
                    let tag_id: i64 =
                        conn.query_row("SELECT id FROM product_tags WHERE name=?1", [&tag], |r| {
                            r.get(0)
                        })?;
                    conn.execute(
                            "INSERT OR IGNORE INTO product_tag_associations (product_id, tag_id) VALUES (?1,?2)",
                            params![id, tag_id],
                        )?;
                    let msg = format!("Added tag '{}' to product {}", tag, id);
                    if json {
                        print_success_json(Success::ok(msg.clone()));
                    } else {
                        quiet_print(&msg, quiet);
                    }
                }
                ProductTagModifyAction::Remove { id, tag } => {
                    let tag_id: Option<i64> = conn
                        .query_row("SELECT id FROM product_tags WHERE name=?1", [&tag], |r| {
                            r.get(0)
                        })
                        .optional()?;
                    if let Some(tid) = tag_id {
                        conn.execute(
                                "DELETE FROM product_tag_associations WHERE product_id=?1 AND tag_id=?2",
                                params![id, tid],
                            )?;
                    }
                    let msg = format!("Removed tag '{}' from product {}", tag, id);
                    if json {
                        print_success_json(Success::ok(msg.clone()));
                    } else {
                        quiet_print(&msg, quiet);
                    }
                }
            },
            ProductAction::Delete { id, force } => {
                // check purchases
                let purch_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM purchases WHERE product_id = ?",
                    [id],
                    |r| r.get(0),
                )?;
                if purch_count > 0 && !force {
                    let err = NutlogError::ProductHasPurchases(id);
                    if json {
                        print_error_json(&err.to_string());
                    } else {
                        eprintln!("{}", err);
                    }
                    std::process::exit(1);
                }
                if force {
                    conn.execute("DELETE FROM purchases WHERE product_id = ?", [id])?;
                }
                // also remove nutrition etc via cascade mostly
                let affected = conn.execute("DELETE FROM products WHERE id = ?", [id])?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("product not found: {}", id));
                    } else {
                        eprintln!("product not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted product {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            ProductAction::Nutrition { action } => match action {
                NutritionAction::Set(args) => {
                    let pid = args.id;
                    set_nutrition(conn, args)?;
                    let msg = format!("Nutrition set for product {}", pid);
                    if json {
                        print_success_json(Success::ok(msg.clone()));
                    } else {
                        quiet_print(&msg, quiet);
                    }
                }
            },
        }
        Ok(())
    }

    fn load_full_product(conn: &Connection, id: i64) -> NutResult<Product> {
        let mut stmt =
            conn.prepare("SELECT id, name, created_at, updated_at FROM products WHERE id = ?")?;
        let p = stmt.query_row([id], |r| row_to_product(conn, r))?;
        Ok(p)
    }

    fn ensure_product_tag(conn: &Connection, name: &str, now: &str) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT OR IGNORE INTO product_tags (name, created_at) VALUES (?1, ?2)",
            params![name, now],
        )?;
        Ok(())
    }

    /// Ensure a nutrient row exists (by name, case-insensitive lookup).
    /// If missing, insert it using the caller's provided name casing and the suggested unit
    /// as its canonical unit (recommended_intake left NULL). Returns the nutrient id.
    fn ensure_nutrient(conn: &Connection, name: &str, suggested_unit: &str) -> NutResult<i64> {
        if let Some(id) = conn
            .query_row(
                "SELECT id FROM nutrients WHERE name = ?1 COLLATE NOCASE",
                [name],
                |r| r.get(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        let now = now_utc();
        conn.execute(
            "INSERT INTO nutrients (name, unit, recommended_intake, created_at) VALUES (?1, ?2, NULL, ?3)",
            params![name, suggested_unit, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn set_nutrition(conn: &mut Connection, args: NutritionSetArgs) -> NutResult<()> {
        // verify product exists (outside tx for early exit; the write path will be atomic)
        let exists: i64 =
            conn.query_row("SELECT COUNT(*) FROM products WHERE id=?", [args.id], |r| {
                r.get(0)
            })?;
        if exists == 0 {
            return Err(NutlogError::ProductNotFound(args.id));
        }

        // Determine the complete nutrition data from either --json-file or the flag arguments.
        let (
            ref_qty,
            ref_unit,
            energy_kcal,
            protein_g,
            carbohydrates_g,
            fat_g,
            fiber_g,
            sugars_g,
            micros_to_set, // Vec<(name, amount, unit)>
        ) = if let Some(path) = &args.json_file {
            let content = std::fs::read_to_string(path).map_err(|e| {
                NutlogError::InvalidNutrition(format!(
                    "failed to read nutrition file '{}': {}",
                    path, e
                ))
            })?;
            let input: NutritionInput = serde_json::from_str(&content).map_err(|e| {
                NutlogError::InvalidNutrition(format!(
                    "invalid nutrition JSON in '{}': {}",
                    path, e
                ))
            })?;
            let mvec: Vec<(String, f64, String)> = input
                .micronutrients
                .into_iter()
                .map(|mi| (mi.name, mi.amount, mi.unit))
                .collect();
            (
                input.reference.quantity,
                input.reference.unit,
                input.energy_kcal,
                input.protein_g,
                input.carbohydrates_g,
                input.fat_g,
                input.fiber_g,
                input.sugars_g,
                mvec,
            )
        } else {
            let rq = args.reference_quantity.ok_or_else(|| {
                    NutlogError::InvalidNutrition(
                        "--reference-quantity and --reference-unit are required (unless --json-file supplies the full payload)".to_string(),
                    )
                })?;
            let ru = args.reference_unit.ok_or_else(|| {
                    NutlogError::InvalidNutrition(
                        "--reference-quantity and --reference-unit are required (unless --json-file supplies the full payload)".to_string(),
                    )
                })?;

            let mut mvec: Vec<(String, f64, String)> = vec![];
            let chunks = &args.micronutrient;
            if !chunks.is_empty() && chunks.len() % 3 != 0 {
                return Err(NutlogError::InvalidNutrition(
                        "invalid --micronutrient usage: each flag must be followed by exactly NAME AMOUNT UNIT (3 values)".to_string(),
                    ));
            }
            for chunk in chunks.chunks_exact(3) {
                let name = chunk[0].clone();
                let amt_str = &chunk[1];
                let unit = chunk[2].clone();
                let amt: f64 = amt_str.parse().map_err(|_| {
                    NutlogError::InvalidNutrition(format!(
                        "invalid amount '{}' for micronutrient '{}'; expected a number",
                        amt_str, name
                    ))
                })?;
                if !amt.is_finite() || amt < 0.0 {
                    return Err(NutlogError::InvalidNutrition(format!(
                        "micronutrient amount must be finite and >= 0 for '{}'",
                        name
                    )));
                }
                mvec.push((name, amt, unit));
            }
            (
                rq,
                ru,
                args.energy_kcal,
                args.protein_g,
                args.carbohydrates_g,
                args.fat_g,
                args.fiber_g,
                args.sugars_g,
                mvec,
            )
        };

        // Apply everything atomically so a "set" either fully succeeds or leaves the prior state.
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO product_nutritions (product_id, reference_quantity, reference_unit, energy_kcal, protein_g, carbohydrates_g, fat_g, fiber_g, sugars_g)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(product_id) DO UPDATE SET
               reference_quantity=excluded.reference_quantity,
               reference_unit=excluded.reference_unit,
               energy_kcal=excluded.energy_kcal,
               protein_g=excluded.protein_g,
               carbohydrates_g=excluded.carbohydrates_g,
               fat_g=excluded.fat_g,
               fiber_g=excluded.fiber_g,
               sugars_g=excluded.sugars_g",
            params![
                args.id, ref_qty, ref_unit,
                energy_kcal, protein_g, carbohydrates_g,
                fat_g, fiber_g, sugars_g
            ],
        )?;

        // Replace semantics for micronutrients: the provided list (possibly empty) becomes the exact set.
        tx.execute(
            "DELETE FROM product_micronutrients WHERE product_id = ?",
            [args.id],
        )?;
        for (name, amt, unit) in micros_to_set {
            let nid = ensure_nutrient(&tx, &name, &unit)?;
            tx.execute(
                "INSERT INTO product_micronutrients (product_id, nutrient_id, amount, unit)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(product_id, nutrient_id) DO UPDATE SET amount=excluded.amount, unit=excluded.unit",
                params![args.id, nid, amt, unit],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    // ---------- NUTRIENT ----------

    pub fn handle_nutrient(
        conn: &Connection,
        action: NutrientAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            NutrientAction::List => {
                let mut stmt = conn.prepare("SELECT id, name, unit, recommended_intake, created_at FROM nutrients ORDER BY name")?;
                let mut list = vec![];
                for row in stmt.query_map([], |r| {
                    let id: i64 = r.get(0)?;
                    let name: String = r.get(1)?;
                    let unit: String = r.get(2)?;
                    let rec: Option<f64> = r.get(3)?;
                    let cat: String = r.get(4)?;
                    Ok(Nutrient {
                        id,
                        name,
                        unit,
                        recommended_intake: rec,
                        created_at: crate::db::make_timestamp_info(&cat),
                    })
                })? {
                    list.push(row?);
                }
                if json {
                    print_json(&list);
                } else {
                    for n in &list {
                        let rec = n
                            .recommended_intake
                            .map(|v| format!(" rec:{}", v))
                            .unwrap_or_default();
                        println!("{}: {} ({}{})", n.id, n.name, n.unit, rec);
                    }
                }
            }
            NutrientAction::Create {
                name,
                unit,
                recommended_intake,
            } => {
                let now = now_utc();
                conn.execute(
                    "INSERT INTO nutrients (name, unit, recommended_intake, created_at) VALUES (?1,?2,?3,?4)",
                    params![name, unit, recommended_intake, now],
                )?;
                let id = conn.last_insert_rowid();
                let msg = format!("Created nutrient {} ({})", id, name);
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            NutrientAction::Show { id } => {
                let n: Option<Nutrient> = conn.query_row(
                    "SELECT id, name, unit, recommended_intake, created_at FROM nutrients WHERE id=?",
                    [id],
                    |r| Ok(Nutrient {
                        id: r.get(0)?, name: r.get(1)?, unit: r.get(2)?, recommended_intake: r.get(3)?,
                        created_at: crate::db::make_timestamp_info(&r.get::<_,String>(4)?),
                    })
                ).optional()?;
                match n {
                    Some(nut) => {
                        if json {
                            print_json(&nut);
                        } else {
                            println!("{:?}", nut);
                        }
                    }
                    None => {
                        if json {
                            print_error_json(&format!("nutrient not found: {}", id));
                        } else {
                            eprintln!("not found");
                        }
                        std::process::exit(1);
                    }
                }
            }
            NutrientAction::Search { query } => {
                let mut stmt = conn.prepare("SELECT id, name FROM nutrients")?;
                let cands: Vec<(i64, String)> = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .filter_map(Result::ok)
                    .collect();
                let ranked = fuzzy_rank(cands, &query);
                let mut out = vec![];
                for (id, _name, _s) in ranked {
                    if let Ok(n) = conn.query_row(
                        "SELECT id, name, unit, recommended_intake, created_at FROM nutrients WHERE id=?",
                        [id],
                        |r| Ok(Nutrient { id, name: r.get(1)?, unit: r.get(2)?, recommended_intake: r.get(3)?, created_at: crate::db::make_timestamp_info(&r.get::<_,String>(4)?) })
                    ) {
                        out.push(n);
                    }
                }
                if json {
                    print_json(&out);
                } else {
                    for n in &out {
                        println!("{}: {} ({})", n.id, n.name, n.unit);
                    }
                }
            }
            NutrientAction::Delete { id, force } => {
                let ref_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM product_micronutrients WHERE nutrient_id = ?",
                    [id],
                    |r| r.get(0),
                )?;
                if ref_count > 0 && !force {
                    let err = NutlogError::NutrientHasReferences(id);
                    if json {
                        print_error_json(&err.to_string());
                    } else {
                        eprintln!("{}", err);
                    }
                    std::process::exit(1);
                }
                let affected = conn.execute("DELETE FROM nutrients WHERE id = ?", [id])?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("nutrient not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted nutrient {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    // ---------- PRODUCT TAG ----------

    pub fn handle_product_tag(
        conn: &Connection,
        action: ProductTagAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            ProductTagAction::Create { name } => {
                let now = now_utc();
                conn.execute(
                    "INSERT OR IGNORE INTO product_tags (name, created_at) VALUES (?1, ?2)",
                    params![name, now],
                )?;
                let id: i64 =
                    conn.query_row("SELECT id FROM product_tags WHERE name=?1", [&name], |r| {
                        r.get(0)
                    })?;
                let msg = format!("Created product-tag {} ({})", id, name);
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            ProductTagAction::List => {
                let mut stmt = conn.prepare(
                    "SELECT pt.id, pt.name, pt.created_at, COUNT(pta.product_id)
                     FROM product_tags pt
                     LEFT JOIN product_tag_associations pta ON pta.tag_id=pt.id
                     GROUP BY pt.id ORDER BY pt.name",
                )?;
                let mut tags = vec![];
                for row in stmt.query_map([], |r| {
                    Ok(Tag {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        created_at: crate::db::make_timestamp_info(&r.get::<_, String>(2)?),
                        usage_count: Some(r.get(3)?),
                    })
                })? {
                    tags.push(row?);
                }
                if json {
                    print_json(&tags);
                } else {
                    for t in &tags {
                        println!(
                            "{}: {} (used by {} products)",
                            t.id,
                            t.name,
                            t.usage_count.unwrap_or(0)
                        );
                    }
                }
            }
            ProductTagAction::Search { query } => {
                let mut stmt = conn.prepare("SELECT id, name FROM product_tags")?;
                let cands: Vec<_> = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .filter_map(|x| x.ok())
                    .collect();
                let ranked = fuzzy_rank(cands, &query);
                let mut out = vec![];
                for (id, _, _) in ranked {
                    let t: Tag = conn.query_row(
                        "SELECT pt.id, pt.name, pt.created_at, COUNT(pta.product_id) FROM product_tags pt LEFT JOIN product_tag_associations pta ON pta.tag_id=pt.id WHERE pt.id=? GROUP BY pt.id",
                        [id],
                        |r| Ok(Tag { id, name: r.get(1)?, created_at: crate::db::make_timestamp_info(&r.get::<_,String>(2)?), usage_count: Some(r.get(3)?) })
                    )?;
                    out.push(t);
                }
                if json {
                    print_json(&out);
                } else {
                    for t in &out {
                        println!("{}: {}", t.id, t.name);
                    }
                }
            }
            ProductTagAction::Show { id } => {
                let tag: Option<Tag> = conn.query_row(
                    "SELECT pt.id, pt.name, pt.created_at, COUNT(pta.product_id) FROM product_tags pt LEFT JOIN product_tag_associations pta ON pta.tag_id = pt.id WHERE pt.id = ? GROUP BY pt.id",
                    [id],
                    |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, created_at: crate::db::make_timestamp_info(&r.get::<_, String>(2)?), usage_count: Some(r.get(3)?) })
                ).optional()?;
                match tag {
                    Some(t) => {
                        if json {
                            print_json(&t);
                        } else {
                            println!(
                                "Tag {}: {} used by {} products",
                                t.id,
                                t.name,
                                t.usage_count.unwrap_or(0)
                            );
                        }
                    }
                    None => {
                        if json {
                            print_error_json(&format!("product tag not found: {}", id));
                        } else {
                            eprintln!("not found");
                        }
                        std::process::exit(1);
                    }
                }
            }
            ProductTagAction::Delete { id } => {
                let affected = conn.execute("DELETE FROM product_tags WHERE id=?", [id])?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("product tag not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted product-tag {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    // ---------- PURCHASE ----------

    pub fn handle_purchase(
        conn: &Connection,
        action: PurchaseAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            PurchaseAction::Create {
                product_id,
                price,
                store,
                date,
                quantity,
            } => {
                // validate product
                let prod_exists: i64 = conn
                    .query_row("SELECT 1 FROM products WHERE id=?", [product_id], |r| {
                        r.get(0)
                    })
                    .optional()?
                    .unwrap_or(0);
                if prod_exists == 0 {
                    if json {
                        print_error_json(&format!("product not found: {}", product_id));
                    } else {
                        eprintln!("product not found");
                    }
                    std::process::exit(1);
                }
                let price_cents = match price {
                    Some(pstr) => Some(parse_price_to_cents(&pstr)?),
                    None => None,
                };
                let purchased_at = parse_flexible_date(&date)
                    .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

                // validate store if given
                if let Some(sid) = store {
                    let s_ok: i64 = conn
                        .query_row("SELECT 1 FROM stores WHERE id=?", [sid], |r| r.get(0))
                        .optional()?
                        .unwrap_or(0);
                    if s_ok == 0 {
                        if json {
                            print_error_json(&format!("store not found: {}", sid));
                        } else {
                            eprintln!("store not found");
                        }
                        std::process::exit(1);
                    }
                }

                let now = now_utc();
                conn.execute(
                    "INSERT INTO purchases (product_id, quantity, price_cents, store_id, purchased_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![product_id, quantity, price_cents, store, purchased_at, now],
                )?;
                let id = conn.last_insert_rowid();

                let msg = format!(
                    "Recorded purchase {} of product {} (qty {})",
                    id, product_id, quantity
                );
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            PurchaseAction::List {
                since,
                until,
                product,
                store,
            } => {
                let mut sql = String::from(
                    "SELECT pu.id, pu.product_id, p.name, pu.quantity, pu.price_cents, pu.store_id, s.name, pu.purchased_at, pu.created_at
                     FROM purchases pu
                     JOIN products p ON p.id = pu.product_id
                     LEFT JOIN stores s ON s.id = pu.store_id
                     WHERE 1=1 "
                );
                let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
                let mut idx = 1;

                if let Some(ref sd) = since {
                    let dt = parse_flexible_date(sd)
                        .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    sql.push_str(&format!(" AND pu.purchased_at >= ?{} ", idx));
                    params_vec.push(Box::new(dt));
                    idx += 1;
                }
                if let Some(ref ud) = until {
                    let dt = parse_flexible_date_bound(ud, DateBound::End)
                        .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    sql.push_str(&format!(" AND pu.purchased_at <= ?{} ", idx));
                    params_vec.push(Box::new(dt));
                    idx += 1;
                }
                if let Some(pid) = product {
                    sql.push_str(&format!(" AND pu.product_id = ?{} ", idx));
                    params_vec.push(Box::new(pid));
                    idx += 1;
                }
                if let Some(sid) = store {
                    sql.push_str(&format!(" AND pu.store_id = ?{} ", idx));
                    params_vec.push(Box::new(sid));
                }
                sql.push_str(" ORDER BY pu.purchased_at DESC, pu.id DESC");

                let mut stmt = conn.prepare(&sql)?;
                let param_refs: Vec<&dyn rusqlite::ToSql> =
                    params_vec.iter().map(|b| b.as_ref()).collect();

                let mut purchases = vec![];
                for row in stmt.query_map(param_refs.as_slice(), |r| {
                    let id: i64 = r.get(0)?;
                    let product_id: i64 = r.get(1)?;
                    let product_name: String = r.get(2)?;
                    let quantity: f64 = r.get(3)?;
                    let price_cents: Option<i64> = r.get(4)?;
                    let store_id: Option<i64> = r.get(5)?;
                    let store_name: Option<String> = r.get(6)?;
                    let purch_at: String = r.get(7)?;
                    let created_at: String = r.get(8)?;
                    Ok(Purchase {
                        id,
                        product_id,
                        product_name,
                        quantity,
                        price_cents,
                        price: format_price_opt(price_cents),
                        store_id,
                        store_name,
                        purchased_at: crate::db::make_timestamp_info(&purch_at),
                        created_at: crate::db::make_timestamp_info(&created_at),
                    })
                })? {
                    purchases.push(row?);
                }

                if json {
                    print_json(&purchases);
                } else {
                    if purchases.is_empty() {
                        println!("(no purchases)");
                        return Ok(());
                    }
                    let mut table = Table::new();
                    table.load_preset(presets::UTF8_FULL_CONDENSED);
                    table.set_header(vec!["ID", "Date", "Product", "Qty", "Price", "Store"]);
                    for pu in &purchases {
                        table.add_row(vec![
                            Cell::new(pu.id),
                            Cell::new(format_local(&pu.purchased_at.utc)),
                            Cell::new(&pu.product_name),
                            Cell::new(pu.quantity),
                            Cell::new(pu.price.as_deref().unwrap_or("-")),
                            Cell::new(pu.store_name.as_deref().unwrap_or("-")),
                        ]);
                    }
                    println!("{}", table);
                }
            }
            PurchaseAction::Show { id } => {
                let p: Option<Purchase> = {
                    let mut stmt = conn.prepare(
                        "SELECT pu.id, pu.product_id, p.name, pu.quantity, pu.price_cents, pu.store_id, s.name, pu.purchased_at, pu.created_at
                         FROM purchases pu JOIN products p ON p.id=pu.product_id LEFT JOIN stores s ON s.id=pu.store_id WHERE pu.id=?"
                    )?;
                    stmt.query_row([id], |r| {
                        Ok(Purchase {
                            id: r.get(0)?,
                            product_id: r.get(1)?,
                            product_name: r.get(2)?,
                            quantity: r.get(3)?,
                            price_cents: r.get(4)?,
                            price: format_price_opt(r.get(4)?),
                            store_id: r.get(5)?,
                            store_name: r.get(6)?,
                            purchased_at: crate::db::make_timestamp_info(&r.get::<_, String>(7)?),
                            created_at: crate::db::make_timestamp_info(&r.get::<_, String>(8)?),
                        })
                    })
                    .optional()?
                };
                match p {
                    Some(pu) => {
                        if json {
                            print_json(&pu);
                        } else {
                            println!(
                                "Purchase {}: {} x {} @ {} on {}",
                                pu.id,
                                pu.quantity,
                                pu.product_name,
                                pu.price.as_deref().unwrap_or("no price"),
                                format_local(&pu.purchased_at.utc)
                            );
                        }
                    }
                    None => {
                        if json {
                            print_error_json(&format!("purchase not found: {}", id));
                        } else {
                            eprintln!("not found");
                        }
                        std::process::exit(1);
                    }
                }
            }
            PurchaseAction::Delete { id } => {
                let affected = conn.execute("DELETE FROM purchases WHERE id = ?", [id])?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("purchase not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted purchase {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    // ---------- STORE ----------

    pub fn handle_store(
        conn: &Connection,
        action: StoreAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            StoreAction::Create { name } => {
                let now = now_utc();
                conn.execute(
                    "INSERT INTO stores (name, created_at) VALUES (?1, ?2)",
                    params![name, now],
                )?;
                let id = conn.last_insert_rowid();
                let msg = format!("Created store {} ({})", id, name);
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            StoreAction::List => {
                let mut stmt = conn.prepare(
                    "SELECT s.id, s.name, s.created_at, GROUP_CONCAT(st.name, ',')
                     FROM stores s
                     LEFT JOIN store_tag_associations sta ON sta.store_id = s.id
                     LEFT JOIN store_tags st ON st.id = sta.tag_id
                     GROUP BY s.id ORDER BY s.id DESC",
                )?;
                let mut stores = vec![];
                for row in stmt.query_map([], |r| {
                    let tags_str: Option<String> = r.get(3)?;
                    Ok(Store {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        tags: tags_str
                            .map(|t| t.split(',').map(|s| s.to_string()).collect())
                            .unwrap_or_default(),
                        created_at: crate::db::make_timestamp_info(&r.get::<_, String>(2)?),
                    })
                })? {
                    stores.push(row?);
                }
                if json {
                    print_json(&stores);
                } else {
                    for s in &stores {
                        println!("{}: {} [{}]", s.id, s.name, s.tags.join(","));
                    }
                }
            }
            StoreAction::Show { id } => {
                let s: Option<Store> = {
                    let mut stmt = conn.prepare(
                        "SELECT s.id, s.name, s.created_at, GROUP_CONCAT(st.name, ',')
                         FROM stores s LEFT JOIN store_tag_associations sta ON sta.store_id=s.id LEFT JOIN store_tags st ON st.id=sta.tag_id
                         WHERE s.id=? GROUP BY s.id"
                    )?;
                    stmt.query_row([id], |r| {
                        let tags_str: Option<String> = r.get(3)?;
                        Ok(Store {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            created_at: crate::db::make_timestamp_info(&r.get::<_, String>(2)?),
                            tags: tags_str
                                .map(|t| t.split(',').map(str::to_string).collect())
                                .unwrap_or_default(),
                        })
                    })
                    .optional()?
                };
                match s {
                    Some(st) => {
                        if json {
                            print_json(&st);
                        } else {
                            println!("Store {}: {} tags: {:?}", st.id, st.name, st.tags);
                        }
                    }
                    None => {
                        if json {
                            print_error_json(&format!("store not found: {}", id));
                        } else {
                            eprintln!("not found");
                        }
                        std::process::exit(1);
                    }
                }
            }
            StoreAction::Rename { id, name } => {
                let aff =
                    conn.execute("UPDATE stores SET name=?1 WHERE id=?2", params![name, id])?;
                if aff == 0 {
                    if json {
                        print_error_json(&format!("store not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Renamed store {} to {}", id, name);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            StoreAction::Tag { action } => match action {
                StoreTagModifyAction::Add { id, tag } => {
                    let now = now_utc();
                    ensure_store_tag(conn, &tag, &now)?;
                    let tid: i64 =
                        conn.query_row("SELECT id FROM store_tags WHERE name=?", [&tag], |r| {
                            r.get(0)
                        })?;
                    conn.execute("INSERT OR IGNORE INTO store_tag_associations (store_id, tag_id) VALUES (?,?)", params![id, tid])?;
                    let msg = format!("Added tag '{}' to store {}", tag, id);
                    if json {
                        print_success_json(Success::ok(msg.clone()));
                    } else {
                        quiet_print(&msg, quiet);
                    }
                }
                StoreTagModifyAction::Remove { id, tag } => {
                    let tid: Option<i64> = conn
                        .query_row("SELECT id FROM store_tags WHERE name=?", [&tag], |r| {
                            r.get(0)
                        })
                        .optional()?;
                    if let Some(tid) = tid {
                        conn.execute(
                            "DELETE FROM store_tag_associations WHERE store_id=? AND tag_id=?",
                            params![id, tid],
                        )?;
                    }
                    let msg = format!("Removed tag '{}' from store {}", tag, id);
                    if json {
                        print_success_json(Success::ok(msg.clone()));
                    } else {
                        quiet_print(&msg, quiet);
                    }
                }
            },
            StoreAction::Delete { id } => {
                let aff = conn.execute("DELETE FROM stores WHERE id=?", [id])?;
                if aff == 0 {
                    if json {
                        print_error_json(&format!("store not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted store {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    fn ensure_store_tag(conn: &Connection, name: &str, now: &str) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT OR IGNORE INTO store_tags (name, created_at) VALUES (?1, ?2)",
            params![name, now],
        )?;
        Ok(())
    }

    // ---------- STORE TAG ----------

    pub fn handle_store_tag(
        conn: &Connection,
        action: StoreTagAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            StoreTagAction::Create { name } => {
                let now = now_utc();
                conn.execute(
                    "INSERT OR IGNORE INTO store_tags (name, created_at) VALUES (?1,?2)",
                    params![name, now],
                )?;
                let id: i64 =
                    conn.query_row("SELECT id FROM store_tags WHERE name=?", [&name], |r| {
                        r.get(0)
                    })?;
                let msg = format!("Created store-tag {} ({})", id, name);
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            StoreTagAction::List => {
                let mut stmt = conn.prepare("SELECT st.id, st.name, st.created_at, COUNT(sta.store_id) FROM store_tags st LEFT JOIN store_tag_associations sta ON sta.tag_id=st.id GROUP BY st.id ORDER BY st.name")?;
                let mut tags = vec![];
                for row in stmt.query_map([], |r| {
                    Ok(Tag {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        created_at: crate::db::make_timestamp_info(&r.get::<_, String>(2)?),
                        usage_count: Some(r.get(3)?),
                    })
                })? {
                    tags.push(row?);
                }
                if json {
                    print_json(&tags);
                } else {
                    for t in &tags {
                        println!(
                            "{}: {} ({} stores)",
                            t.id,
                            t.name,
                            t.usage_count.unwrap_or(0)
                        );
                    }
                }
            }
            StoreTagAction::Search { query } => {
                let mut stmt = conn.prepare("SELECT id, name FROM store_tags")?;
                let cands: Vec<_> = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .filter_map(|x| x.ok())
                    .collect();
                let ranked = fuzzy_rank(cands, &query);
                let mut out = vec![];
                for (id, _, _) in ranked {
                    let t = conn.query_row("SELECT st.id, st.name, st.created_at, COUNT(sta.store_id) FROM store_tags st LEFT JOIN store_tag_associations sta ON sta.tag_id = st.id WHERE st.id = ? GROUP BY st.id", [id], |r| Ok(Tag { id, name: r.get(1)?, created_at: crate::db::make_timestamp_info(&r.get::<_,String>(2)?), usage_count: Some(r.get(3)?) }))?;
                    out.push(t);
                }
                if json {
                    print_json(&out);
                } else {
                    for t in &out {
                        println!("{}: {}", t.id, t.name);
                    }
                }
            }
            StoreTagAction::Show { id } => {
                let t: Option<Tag> = conn.query_row(
                    "SELECT st.id, st.name, st.created_at, COUNT(sta.store_id) FROM store_tags st LEFT JOIN store_tag_associations sta ON sta.tag_id=st.id WHERE st.id=? GROUP BY st.id",
                    [id],
                    |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, created_at: crate::db::make_timestamp_info(&r.get::<_,String>(2)?), usage_count: Some(r.get(3)?) })
                ).optional()?;
                match t {
                    Some(tg) => {
                        if json {
                            print_json(&tg);
                        } else {
                            println!("Store tag {}: {}", tg.id, tg.name);
                        }
                    }
                    None => {
                        if json {
                            print_error_json(&format!("store tag not found: {}", id));
                        } else {
                            eprintln!("not found");
                        }
                        std::process::exit(1);
                    }
                }
            }
            StoreTagAction::Delete { id } => {
                let aff = conn.execute("DELETE FROM store_tags WHERE id=?", [id])?;
                if aff == 0 {
                    if json {
                        print_error_json(&format!("store tag not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted store-tag {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    // ---------- CONSUMPTION ----------

    pub fn handle_consumption(
        conn: &Connection,
        action: ConsumptionAction,
        json: bool,
        quiet: bool,
    ) -> Result<()> {
        match action {
            ConsumptionAction::Create {
                product_id,
                quantity,
                unit,
                date,
            } => {
                let exists: Option<i64> = conn
                    .query_row("SELECT id FROM products WHERE id=?", [product_id], |r| {
                        r.get(0)
                    })
                    .optional()?;
                if exists.is_none() {
                    if json {
                        print_error_json(&format!("product not found: {}", product_id));
                    } else {
                        eprintln!("product not found");
                    }
                    std::process::exit(1);
                }
                let qty = match quantity {
                    Some(q) => q,
                    None => {
                        // try suggest from product ref
                        if let Ok(Some(nut)) = load_nutrition(conn, product_id) {
                            nut.reference.quantity
                        } else {
                            1.0
                        }
                    }
                };
                let consumed_at = parse_flexible_date(&date)
                    .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let now = now_utc();
                conn.execute(
                    "INSERT INTO consumptions (product_id, quantity, unit, consumed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![product_id, qty, unit, consumed_at, now],
                )?;
                let id = conn.last_insert_rowid();
                let msg = format!(
                    "Recorded consumption {} of product {} ({} {})",
                    id,
                    product_id,
                    qty,
                    unit.as_deref().unwrap_or("")
                );
                if json {
                    print_success_json(Success::created(id, msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
            ConsumptionAction::List {
                since,
                until,
                product,
            } => {
                let mut sql = "SELECT c.id, c.product_id, p.name, c.quantity, c.unit, c.consumed_at, c.created_at
                               FROM consumptions c JOIN products p ON p.id = c.product_id WHERE 1=1 ".to_string();
                let mut pvec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
                let mut i = 1;
                if let Some(sd) = since {
                    let dt = parse_flexible_date(&sd)
                        .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    sql.push_str(&format!(" AND c.consumed_at >= ?{} ", i));
                    pvec.push(Box::new(dt));
                    i += 1;
                }
                if let Some(ud) = until {
                    let dt = parse_flexible_date_bound(&ud, DateBound::End)
                        .map_err(|e| NutlogError::InvalidDate(e.to_string()))?
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    sql.push_str(&format!(" AND c.consumed_at <= ?{} ", i));
                    pvec.push(Box::new(dt));
                    i += 1;
                }
                if let Some(pid) = product {
                    sql.push_str(&format!(" AND c.product_id = ?{} ", i));
                    pvec.push(Box::new(pid));
                }
                sql.push_str(" ORDER BY c.consumed_at DESC");

                let mut stmt = conn.prepare(&sql)?;
                let refs: Vec<&dyn rusqlite::ToSql> = pvec.iter().map(|b| b.as_ref()).collect();
                let mut cons = vec![];
                for row in stmt.query_map(refs.as_slice(), |r| {
                    Ok(Consumption {
                        id: r.get(0)?,
                        product_id: r.get(1)?,
                        product_name: r.get(2)?,
                        quantity: r.get(3)?,
                        unit: r.get(4)?,
                        consumed_at: crate::db::make_timestamp_info(&r.get::<_, String>(5)?),
                        created_at: crate::db::make_timestamp_info(&r.get::<_, String>(6)?),
                    })
                })? {
                    cons.push(row?);
                }

                if json {
                    print_json(&cons);
                } else {
                    for c in &cons {
                        println!(
                            "{}: {} {} of {} @ {}",
                            c.id,
                            c.quantity,
                            c.unit.as_deref().unwrap_or(""),
                            c.product_name,
                            format_local(&c.consumed_at.utc)
                        );
                    }
                }
            }
            ConsumptionAction::Delete { id } => {
                let affected = conn.execute("DELETE FROM consumptions WHERE id = ?", [id])?;
                if affected == 0 {
                    if json {
                        print_error_json(&format!("consumption not found: {}", id));
                    } else {
                        eprintln!("not found");
                    }
                    std::process::exit(1);
                }
                let msg = format!("Deleted consumption {}", id);
                if json {
                    print_success_json(Success::ok(msg.clone()));
                } else {
                    quiet_print(&msg, quiet);
                }
            }
        }
        Ok(())
    }

    // ---------- REPORT ----------

    struct NutritionConsumptionRow {
        consumed_at: String,
        product_id: i64,
        scale: f64,
        energy_kcal: Option<f64>,
        protein_g: Option<f64>,
        carbohydrates_g: Option<f64>,
        fat_g: Option<f64>,
        fiber_g: Option<f64>,
        sugars_g: Option<f64>,
    }

    fn period_from_resolved(resolved: &ResolvedNutritionPeriod) -> Period {
        Period {
            since: resolved.since_label.clone(),
            until: resolved.until_label.clone(),
            days: resolved.days,
        }
    }

    fn add_row_macros(totals: &mut MacroTotals, row: &NutritionConsumptionRow) {
        let scale = row.scale;
        if let Some(v) = row.energy_kcal {
            totals.energy_kcal = Some(totals.energy_kcal.unwrap_or(0.0) + v * scale);
        }
        if let Some(v) = row.protein_g {
            totals.protein_g = Some(totals.protein_g.unwrap_or(0.0) + v * scale);
        }
        if let Some(v) = row.carbohydrates_g {
            totals.carbohydrates_g = Some(totals.carbohydrates_g.unwrap_or(0.0) + v * scale);
        }
        if let Some(v) = row.fat_g {
            totals.fat_g = Some(totals.fat_g.unwrap_or(0.0) + v * scale);
        }
        if let Some(v) = row.fiber_g {
            totals.fiber_g = Some(totals.fiber_g.unwrap_or(0.0) + v * scale);
        }
        if let Some(v) = row.sugars_g {
            totals.sugars_g = Some(totals.sugars_g.unwrap_or(0.0) + v * scale);
        }
    }

    fn fetch_nutrition_consumptions(
        conn: &Connection,
        resolved: &ResolvedNutritionPeriod,
    ) -> Result<Vec<NutritionConsumptionRow>> {
        let mut sql = "SELECT c.quantity, c.unit, pn.reference_quantity, pn.reference_unit,
                              pn.energy_kcal, pn.protein_g, pn.carbohydrates_g, pn.fat_g, pn.fiber_g, pn.sugars_g,
                              c.product_id, c.consumed_at
                       FROM consumptions c
                       JOIN product_nutritions pn ON pn.product_id = c.product_id
                       WHERE 1=1 "
            .to_string();
        let mut pvec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        let mut i = 1;
        if let Some(ref sd) = resolved.since_utc {
            let d = sd.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            sql.push_str(&format!(" AND c.consumed_at >= ?{} ", i));
            pvec.push(Box::new(d));
            i += 1;
        }
        if let Some(ref ud) = resolved.until_utc {
            let d = ud.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            sql.push_str(&format!(" AND c.consumed_at <= ?{} ", i));
            pvec.push(Box::new(d));
        }
        let mut stmt = conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = pvec.iter().map(|b| b.as_ref()).collect();
        let mut rows = vec![];
        for row in stmt.query_map(refs.as_slice(), |r| {
            let cons_qty: f64 = r.get(0)?;
            let ref_qty: f64 = r.get(2)?;
            let scale = if ref_qty > 0.0 {
                cons_qty / ref_qty
            } else {
                0.0
            };
            Ok(NutritionConsumptionRow {
                scale,
                energy_kcal: r.get(4)?,
                protein_g: r.get(5)?,
                carbohydrates_g: r.get(6)?,
                fat_g: r.get(7)?,
                fiber_g: r.get(8)?,
                sugars_g: r.get(9)?,
                product_id: r.get(10)?,
                consumed_at: r.get(11)?,
            })
        })? {
            rows.push(row?);
        }
        Ok(rows)
    }

    fn aggregate_micronutrients(
        conn: &Connection,
        rows: &[NutritionConsumptionRow],
    ) -> Result<Vec<MicroTotal>> {
        let mut micro_map: HashMap<i64, (String, String, f64)> = HashMap::new();
        for row in rows {
            let mut mstmt = conn.prepare(
                "SELECT pm.nutrient_id, pm.amount, pm.unit, n.name
                 FROM product_micronutrients pm
                 JOIN nutrients n ON n.id = pm.nutrient_id
                 WHERE pm.product_id = ?",
            )?;
            for mr in mstmt.query_map([row.product_id], |mr| {
                Ok((
                    mr.get::<_, i64>(0)?,
                    mr.get::<_, f64>(1)? * row.scale,
                    mr.get::<_, String>(2)?,
                    mr.get::<_, String>(3)?,
                ))
            })? {
                let (nid, amt, unit, nm) = mr?;
                let entry = micro_map.entry(nid).or_insert((nm, unit, 0.0));
                entry.2 += amt;
            }
        }
        Ok(micro_map
            .into_iter()
            .map(|(nid, (nm, un, tot))| MicroTotal {
                nutrient_id: nid,
                name: nm,
                unit: un,
                total_amount: tot,
            })
            .collect())
    }

    fn apply_value_filter(totals: MacroTotals, value: NutritionReportValue) -> MacroTotals {
        match value {
            NutritionReportValue::Macronutrients => totals,
            NutritionReportValue::Calories => MacroTotals {
                energy_kcal: totals.energy_kcal,
                ..Default::default()
            },
            NutritionReportValue::Protein => MacroTotals {
                protein_g: totals.protein_g,
                ..Default::default()
            },
            NutritionReportValue::Carbohydrates => MacroTotals {
                carbohydrates_g: totals.carbohydrates_g,
                ..Default::default()
            },
            NutritionReportValue::Fat => MacroTotals {
                fat_g: totals.fat_g,
                ..Default::default()
            },
            NutritionReportValue::Fiber => MacroTotals {
                fiber_g: totals.fiber_g,
                ..Default::default()
            },
            NutritionReportValue::Sugars => MacroTotals {
                sugars_g: totals.sugars_g,
                ..Default::default()
            },
        }
    }

    fn print_macro_totals_human(totals: &MacroTotals, indent: &str) {
        if let Some(v) = totals.energy_kcal {
            println!("{indent}energy: {v:.1} kcal");
        }
        if let Some(v) = totals.protein_g {
            println!("{indent}protein: {v:.1} g");
        }
        if let Some(v) = totals.carbohydrates_g {
            println!("{indent}carbohydrates: {v:.1} g");
        }
        if let Some(v) = totals.fat_g {
            println!("{indent}fat: {v:.1} g");
        }
        if let Some(v) = totals.fiber_g {
            println!("{indent}fiber: {v:.1} g");
        }
        if let Some(v) = totals.sugars_g {
            println!("{indent}sugars: {v:.1} g");
        }
    }

    fn format_single_macro_value(totals: &MacroTotals, value: NutritionReportValue) -> String {
        match value {
            NutritionReportValue::Calories => {
                format!("{:.1} kcal", totals.energy_kcal.unwrap_or(0.0))
            }
            NutritionReportValue::Protein => format!("{:.1} g", totals.protein_g.unwrap_or(0.0)),
            NutritionReportValue::Carbohydrates => {
                format!("{:.1} g", totals.carbohydrates_g.unwrap_or(0.0))
            }
            NutritionReportValue::Fat => format!("{:.1} g", totals.fat_g.unwrap_or(0.0)),
            NutritionReportValue::Fiber => format!("{:.1} g", totals.fiber_g.unwrap_or(0.0)),
            NutritionReportValue::Sugars => format!("{:.1} g", totals.sugars_g.unwrap_or(0.0)),
            NutritionReportValue::Macronutrients => String::new(),
        }
    }

    fn build_daily_entries(
        rows: &[NutritionConsumptionRow],
        fill_range: Option<(chrono::NaiveDate, chrono::NaiveDate)>,
        value: NutritionReportValue,
    ) -> Result<Vec<DailyNutritionEntry>> {
        let mut buckets: BTreeMap<chrono::NaiveDate, (MacroTotals, i64)> = BTreeMap::new();
        for row in rows {
            let day = local_date_from_rfc3339(&row.consumed_at)?;
            let entry = buckets.entry(day).or_default();
            add_row_macros(&mut entry.0, row);
            entry.1 += 1;
        }

        let dates: Vec<chrono::NaiveDate> = if let Some((start, end)) = fill_range {
            let mut d = start;
            let mut out = vec![];
            while d <= end {
                out.push(d);
                d += chrono::Duration::days(1);
            }
            out
        } else {
            buckets.keys().copied().collect()
        };

        Ok(dates
            .into_iter()
            .map(|d| {
                let (totals, count) = buckets.remove(&d).unwrap_or_default();
                DailyNutritionEntry {
                    date: d.format("%Y-%m-%d").to_string(),
                    total_consumed_items: count,
                    totals: apply_value_filter(totals, value),
                }
            })
            .collect())
    }

    fn nutrition_summary(
        conn: &Connection,
        period: &NutritionPeriodArgs,
        json: bool,
    ) -> Result<()> {
        let resolved = resolve_nutrition_period(
            period.since.as_deref(),
            period.until.as_deref(),
            period.days,
        )?;
        let rows = fetch_nutrition_consumptions(conn, &resolved)?;
        let mut totals = MacroTotals::default();
        for row in &rows {
            add_row_macros(&mut totals, row);
        }
        let count = rows.len() as i64;
        let micros = aggregate_micronutrients(conn, &rows)?;

        let report = NutritionReport {
            period: period_from_resolved(&resolved),
            total_consumed_items: count,
            totals,
            micronutrients: micros,
        };

        if json {
            print_json(&report);
        } else {
            println!("Nutrition report ({} items)", count);
            print_macro_totals_human(&report.totals, "  ");
            if !report.micronutrients.is_empty() {
                println!("  key micros:");
                for m in report.micronutrients.iter().take(5) {
                    println!("    {}: {:.2} {}", m.name, m.total_amount, m.unit);
                }
            }
        }
        Ok(())
    }

    fn nutrition_list(conn: &Connection, args: &NutritionListArgs, json: bool) -> Result<()> {
        let resolved = resolve_nutrition_period(
            args.period.since.as_deref(),
            args.period.until.as_deref(),
            args.period.days,
        )?;
        let rows = fetch_nutrition_consumptions(conn, &resolved)?;
        let days = build_daily_entries(&rows, resolved.fill_range, args.value)?;
        let report = NutritionDailyReport {
            period: period_from_resolved(&resolved),
            value: args.value.label().to_string(),
            days,
        };

        if json {
            print_json(&report);
        } else if args.value == NutritionReportValue::Macronutrients {
            println!("Nutrition by day ({})", report.value);
            for day in &report.days {
                println!("  {} ({} items)", day.date, day.total_consumed_items);
                print_macro_totals_human(&day.totals, "    ");
            }
        } else {
            println!("Nutrition by day ({})", report.value);
            println!("  {:<12} {:<12} ITEMS", "DATE", "VALUE");
            for day in &report.days {
                println!(
                    "  {:<12} {:<12} {}",
                    day.date,
                    format_single_macro_value(&day.totals, args.value),
                    day.total_consumed_items
                );
            }
        }
        Ok(())
    }

    pub fn handle_report(
        conn: &Connection,
        action: ReportAction,
        json: bool,
        _quiet: bool,
    ) -> Result<()> {
        match action {
            ReportAction::Nutrition { action } => match action {
                NutritionReportAction::Summary(period) => nutrition_summary(conn, &period, json)?,
                NutritionReportAction::List(args) => nutrition_list(conn, &args, json)?,
            },
            ReportAction::Spending {
                by,
                since,
                until,
                period: _period,
            } => {
                // Simple implementation: total + by store always, by_product if requested.
                let mut base_sql =
                    "SELECT COALESCE(SUM(price_cents),0) FROM purchases pu WHERE 1=1 ".to_string();
                let mut pvec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
                let mut i = 1;
                if let Some(sd) = &since {
                    let d =
                        parse_flexible_date(sd)?.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    base_sql.push_str(&format!(" AND pu.purchased_at >= ?{} ", i));
                    pvec.push(Box::new(d));
                    i += 1;
                }
                if let Some(ud) = &until {
                    let d = parse_flexible_date_bound(ud, DateBound::End)?
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    base_sql.push_str(&format!(" AND pu.purchased_at <= ?{} ", i));
                    pvec.push(Box::new(d));
                }

                let total_cents: i64 = {
                    let mut stmt = conn.prepare(&base_sql)?;
                    let rfs: Vec<&dyn rusqlite::ToSql> = pvec.iter().map(|b| b.as_ref()).collect();
                    stmt.query_row(rfs.as_slice(), |r| r.get(0))?
                };

                let mut by_store = vec![];
                {
                    let mut ssql = "SELECT pu.store_id, COALESCE(s.name, '(no store)'), COALESCE(SUM(pu.price_cents),0), COUNT(*)
                                    FROM purchases pu LEFT JOIN stores s ON s.id = pu.store_id WHERE 1=1 ".to_string();
                    // re-add filters
                    i = 1;
                    let mut spvec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
                    if let Some(sd) = &since {
                        let d = parse_flexible_date(sd)?
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        ssql.push_str(&format!(" AND pu.purchased_at >= ?{} ", i));
                        spvec.push(Box::new(d));
                        i += 1;
                    }
                    if let Some(ud) = &until {
                        let d = parse_flexible_date_bound(ud, DateBound::End)?
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        ssql.push_str(&format!(" AND pu.purchased_at <= ?{} ", i));
                        spvec.push(Box::new(d));
                    }
                    ssql.push_str(" GROUP BY pu.store_id ORDER BY SUM(pu.price_cents) DESC");
                    let mut stmt = conn.prepare(&ssql)?;
                    let rfs: Vec<&dyn rusqlite::ToSql> = spvec.iter().map(|b| b.as_ref()).collect();
                    for row in stmt.query_map(rfs.as_slice(), |r| {
                        let sid: Option<i64> = r.get(0)?;
                        let sname: String = r.get(1)?;
                        let cents: i64 = r.get(2)?;
                        let cnt: i64 = r.get(3)?;
                        Ok(StoreSpending {
                            store_id: sid,
                            store_name: sname,
                            cents,
                            amount: format!("${}", cents_to_str(cents)),
                            purchase_count: cnt,
                        })
                    })? {
                        by_store.push(row?);
                    }
                }

                let mut by_prod = None;
                if by == "product" {
                    let mut psql = "SELECT pu.product_id, p.name, COALESCE(SUM(pu.price_cents),0), COUNT(*)
                                    FROM purchases pu JOIN products p ON p.id=pu.product_id WHERE 1=1 ".to_string();
                    i = 1;
                    let mut ppvec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
                    if let Some(sd) = &since {
                        let d = parse_flexible_date(sd)?
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        psql.push_str(&format!(" AND pu.purchased_at>=?{} ", i));
                        ppvec.push(Box::new(d));
                        i += 1;
                    }
                    if let Some(ud) = &until {
                        let d = parse_flexible_date_bound(ud, DateBound::End)?
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        psql.push_str(&format!(" AND pu.purchased_at<=?{} ", i));
                        ppvec.push(Box::new(d));
                    }
                    psql.push_str(" GROUP BY pu.product_id ORDER BY SUM(pu.price_cents) DESC");
                    let mut stmt = conn.prepare(&psql)?;
                    let rfs: Vec<&dyn rusqlite::ToSql> = ppvec.iter().map(|b| b.as_ref()).collect();
                    let mut prods = vec![];
                    for row in stmt.query_map(rfs.as_slice(), |r| {
                        let pid: i64 = r.get(0)?;
                        let pname: String = r.get(1)?;
                        let cents: i64 = r.get(2)?;
                        let cnt: i64 = r.get(3)?;
                        Ok(ProductSpending {
                            product_id: pid,
                            product_name: pname,
                            cents,
                            amount: format!("${}", cents_to_str(cents)),
                            purchase_count: cnt,
                        })
                    })? {
                        prods.push(row?);
                    }
                    by_prod = Some(prods);
                }

                let report = SpendingReport {
                    period: Period {
                        since: since.clone(),
                        until: until.clone(),
                        days: None,
                    },
                    total_cents,
                    total: format!("${}", cents_to_str(total_cents)),
                    by_store,
                    by_product: by_prod,
                };

                if json {
                    print_json(&report);
                } else {
                    println!("Spending total: {}", report.total);
                    println!("By store:");
                    for s in &report.by_store {
                        println!(
                            "  {}: {} ({} purchases)",
                            s.store_name, s.amount, s.purchase_count
                        );
                    }
                    if let Some(ps) = &report.by_product {
                        println!("By product:");
                        for p in ps {
                            println!("  {}: {} ({}x)", p.product_name, p.amount, p.purchase_count);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod search_tests {
        use super::{fuzzy_rank, name_match_score};

        #[test]
        fn substring_beats_fuzzy_false_positive() {
            let milk_score = name_match_score("Cappuccino (whole milk, no sugar)", "milk");
            let milanesa_score = name_match_score("Milanesa de Ternera Ofe", "milk");
            assert!(milk_score > milanesa_score);
            assert!(milk_score >= 0.9);
            assert_eq!(milanesa_score, 0.0);
        }

        #[test]
        fn prefix_match_for_short_query() {
            assert!(name_match_score("Banana Bunch", "ban") >= 0.85);
        }

        #[test]
        fn multi_word_query_matches_vitamin_d() {
            let score = name_match_score("Vitamin D", "vit d");
            assert!(score >= 0.8);
            assert_eq!(name_match_score("Vitamin B6", "vit d"), 0.0);
            assert_eq!(
                name_match_score("Pantothenic acid (Vitamin B5)", "vit d"),
                0.0
            );
        }

        #[test]
        fn fuzzy_rank_filters_irrelevant_products() {
            let items = vec![
                (37, "Milanesa de Ternera Ofe".into()),
                (15, "Cappuccino (whole milk, no sugar)".into()),
                (1, "Pomelo".into()),
            ];
            let ranked = fuzzy_rank(items, "milk");
            assert_eq!(ranked.len(), 1);
            assert_eq!(ranked[0].0, 15);
        }
    }
}
