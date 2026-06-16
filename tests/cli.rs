use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

fn nutlog_cmd() -> Command {
    Command::cargo_bin("nutlog").unwrap()
}

#[test]
fn help_works() {
    let mut cmd = nutlog_cmd();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("nutlog"));
}

#[test]
fn version_works() {
    let mut cmd = nutlog_cmd();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("nutlog"));
}

#[test]
fn product_create_and_list_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    // create
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Test Yogurt")
        .arg("--tags")
        .arg("yogurt,dairy");
    cmd.assert().success();

    // list json
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("list");
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("\"name\": \"Test Yogurt\""));
    assert!(stdout.contains("\"tags\": [\n      \"dairy\",\n      \"yogurt\""));
}

#[test]
fn product_search_by_name_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Banana Bunch");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("search")
        .arg("--name")
        .arg("ban");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Banana Bunch"));
}

#[test]
fn purchase_with_price_and_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    // product
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Milk 1L");
    cmd.assert().success();

    // purchase $3.49
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("purchase")
        .arg("create")
        .arg("1")
        .arg("--price")
        .arg("3.49")
        .arg("--date")
        .arg("today");
    cmd.assert().success();

    // list json check cents and formatted
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("purchase")
        .arg("list");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"price_cents\": 349"));
    assert!(s.contains("\"price\": \"$3.49\""));
}

#[test]
fn delete_without_force_fails_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("ToDelete");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("purchase")
        .arg("create")
        .arg("1")
        .arg("--date")
        .arg("today");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("delete")
        .arg("1");
    let out = cmd.assert().failure().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": false"));
    assert!(s.contains("has associated purchases"));
}

#[test]
fn nutrient_list_has_prepop() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("list");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Protein"));
    assert!(s.contains("Vitamin"));
}

#[test]
fn report_nutrition_basic() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    // setup product + nutrition + consumption
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Yogurt Pot");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("100")
        .arg("--reference-unit")
        .arg("g")
        .arg("--protein-g")
        .arg("8")
        .arg("--carbohydrates-g")
        .arg("12")
        .arg("--fat-g")
        .arg("5")
        .arg("--fiber-g")
        .arg("2")
        .arg("--sugars-g")
        .arg("3");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("consumption")
        .arg("create")
        .arg("1")
        .arg("--quantity")
        .arg("200")
        .arg("--unit")
        .arg("g");
    cmd.assert().success();

    // JSON report should include all macros including fiber
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("report")
        .arg("nutrition");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    // 200/100 * 8 = 16
    assert!(s.contains("\"protein_g\": 16.0"));
    assert!(s.contains("\"carbohydrates_g\": 24.0"));
    assert!(s.contains("\"fat_g\": 10.0"));
    assert!(s.contains("\"fiber_g\": 4.0"));
    assert!(s.contains("\"sugars_g\": 6.0"));

    // Human output should also show fiber (and other macros)
    let mut cmd = nutlog_cmd();
    cmd.arg("--db").arg(&db).arg("report").arg("nutrition");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("carbohydrates: 24.0 g"));
    assert!(s.contains("fat: 10.0 g"));
    assert!(s.contains("fiber: 4.0 g"));
    assert!(s.contains("sugars: 6.0 g"));
}

#[test]
fn product_nutrition_set_with_micronutrients_flags() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Fish Oil 1000mg");
    cmd.assert().success();

    // Set using the new --micronutrient repeatable flag (mix of seeded + simple macro)
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("1")
        .arg("--reference-unit")
        .arg("capsule")
        .arg("--energy-kcal")
        .arg("10")
        .arg("--micronutrient")
        .arg("Omega 3 EPA")
        .arg("181")
        .arg("mg")
        .arg("--micronutrient")
        .arg("Omega 3 DHA")
        .arg("121")
        .arg("mg");
    cmd.assert().success();

    // Verify via JSON show: micros are present with names, amounts, units and ids
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("show")
        .arg("1");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"reference\""));
    assert!(s.contains("\"unit\": \"capsule\""));
    assert!(s.contains("\"Omega 3 EPA\""));
    assert!(s.contains("\"amount\": 181.0"));
    assert!(s.contains("\"unit\": \"mg\""));
    assert!(s.contains("\"Omega 3 DHA\""));
    assert!(s.contains("\"nutrient_id\""));
}

#[test]
fn product_nutrition_set_via_json_file() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");
    let json_path = dir.path().join("nutrition.json");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Magnesium Tabs");
    cmd.assert().success();

    let payload = r#"{
        "reference": {"quantity": 1.0, "unit": "tablet"},
        "energy_kcal": null,
        "micronutrients": [
            {"name": "Magnesium elemental", "amount": 200.0, "unit": "mg"},
            {"name": "Creatine Monohydrate", "amount": 0.5, "unit": "g"}
        ]
    }"#;
    std::fs::write(&json_path, payload).unwrap();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--json-file")
        .arg(&json_path);
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("show")
        .arg("1");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"Magnesium elemental\""));
    assert!(s.contains("200.0"));
    assert!(s.contains("\"Creatine Monohydrate\""));
    assert!(s.contains("0.5"));
}

#[test]
fn nutrition_set_auto_creates_nutrient() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Special Collagen");
    cmd.assert().success();

    // Use a name that does not exist in the seed list
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("10")
        .arg("--reference-unit")
        .arg("g")
        .arg("--micronutrient")
        .arg("Unicorn Collagen X")
        .arg("7.5")
        .arg("g");
    cmd.assert().success();

    // The nutrient should have been created on the fly
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("list");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Unicorn Collagen X"));
}

#[test]
fn report_nutrition_single_day_range_includes_afternoon_consumption() {
    use chrono::{Local, TimeZone};

    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let today = Local::now().date_naive();
    let date_str = today.format("%Y-%m-%d").to_string();
    let consumed_at = Local
        .from_local_datetime(&today.and_hms_opt(14, 30, 0).unwrap())
        .single()
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Lunch Item");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("100")
        .arg("--reference-unit")
        .arg("g")
        .arg("--protein-g")
        .arg("10");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("consumption")
        .arg("create")
        .arg("1")
        .arg("--quantity")
        .arg("100")
        .arg("--unit")
        .arg("g")
        .arg("--date")
        .arg(&consumed_at);
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("report")
        .arg("nutrition")
        .arg("--since")
        .arg(&date_str)
        .arg("--until")
        .arg(&date_str);
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"total_consumed_items\": 1"));
    assert!(s.contains("\"protein_g\": 10.0"));
}

#[test]
fn report_nutrition_scales_micronutrients() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Creatine Powder");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("5")
        .arg("--reference-unit")
        .arg("g")
        .arg("--micronutrient")
        .arg("Creatine Monohydrate")
        .arg("5")
        .arg("g");
    cmd.assert().success();

    // Consume 10 g (2x the reference of 5 g)
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("consumption")
        .arg("create")
        .arg("1")
        .arg("--quantity")
        .arg("10")
        .arg("--unit")
        .arg("g");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("report")
        .arg("nutrition");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    // 10g / 5g * 5g creatine = 10 g total
    assert!(s.contains("\"Creatine Monohydrate\""));
    assert!(s.contains("\"total_amount\": 10.0"));
}

#[test]
fn product_nutrition_set_replaces_micronutrients() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Multi Supp");
    cmd.assert().success();

    // First set: two micros
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("1")
        .arg("--reference-unit")
        .arg("serving")
        .arg("--micronutrient")
        .arg("Omega 3 EPA")
        .arg("100")
        .arg("mg")
        .arg("--micronutrient")
        .arg("Hyaluronic acid")
        .arg("30")
        .arg("mg");
    cmd.assert().success();

    // Second set: only one different micro (should replace, not append)
    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("1")
        .arg("--reference-unit")
        .arg("serving")
        .arg("--micronutrient")
        .arg("Collagen peptides")
        .arg("2.5")
        .arg("g");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("product")
        .arg("show")
        .arg("1");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"Collagen peptides\""));
    assert!(s.contains("2.5"));
    // The previous ones must be gone
    assert!(!s.contains("Omega 3 EPA"));
    assert!(!s.contains("Hyaluronic acid"));
}

#[test]
fn purchase_delete_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Delete Me Milk");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("purchase")
        .arg("create")
        .arg("1")
        .arg("--date")
        .arg("today");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("purchase")
        .arg("delete")
        .arg("1");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": true"));
    assert!(s.contains("Deleted purchase 1"));

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("purchase")
        .arg("list");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(!s.contains("\"id\": 1"));
}

#[test]
fn consumption_delete_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Snack Bar");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("consumption")
        .arg("create")
        .arg("1")
        .arg("--quantity")
        .arg("50")
        .arg("--unit")
        .arg("g");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("consumption")
        .arg("delete")
        .arg("1");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": true"));
    assert!(s.contains("Deleted consumption 1"));

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("consumption")
        .arg("list");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(!s.contains("\"id\": 1"));
}

#[test]
fn nutrient_delete_unreferenced_json() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("create")
        .arg("Temp Nutrient X")
        .arg("--unit")
        .arg("mg");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": true"));
    let nutrient_id = s
        .split("\"id\":")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .unwrap()
        .trim();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("delete")
        .arg(nutrient_id);
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": true"));
    assert!(s.contains("Deleted nutrient"));
}

#[test]
fn nutrient_delete_without_force_fails_when_referenced() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("create")
        .arg("Fish Oil");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("create")
        .arg("Ref Nutrient Y")
        .arg("--unit")
        .arg("mg");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    let nutrient_id = s
        .split("\"id\":")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .unwrap()
        .trim();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("product")
        .arg("nutrition")
        .arg("set")
        .arg("1")
        .arg("--reference-quantity")
        .arg("1")
        .arg("--reference-unit")
        .arg("capsule")
        .arg("--micronutrient")
        .arg("Ref Nutrient Y")
        .arg("10")
        .arg("mg");
    cmd.assert().success();

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("delete")
        .arg(nutrient_id);
    let out = cmd.assert().failure().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": false"));
    assert!(s.contains("referenced by product nutrition data"));

    let mut cmd = nutlog_cmd();
    cmd.arg("--db")
        .arg(&db)
        .arg("--json")
        .arg("nutrient")
        .arg("delete")
        .arg(nutrient_id)
        .arg("--force");
    let out = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"success\": true"));
    assert!(s.contains("Deleted nutrient"));
}
