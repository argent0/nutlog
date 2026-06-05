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
        .arg("8");
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
}
