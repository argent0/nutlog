use serde::{Deserialize, Serialize};

use crate::db::TimestampInfo;

// Common success envelope for mutating operations when --json
#[derive(Serialize, Debug)]
pub struct Success {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Success {
    pub fn created(id: i64, msg: impl Into<String>) -> Self {
        Self {
            success: true,
            id: Some(id),
            message: Some(msg.into()),
        }
    }
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            id: None,
            message: Some(msg.into()),
        }
    }
    #[allow(dead_code)]
    pub fn fail(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            id: None,
            message: Some(msg.into()),
        }
    }
}

// ---------- Product ----------

#[derive(Serialize, Debug)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub tags: Vec<String>,
    pub nutritional_information: Option<NutritionalInformation>,
    pub created_at: TimestampInfo,
    pub updated_at: TimestampInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NutritionalInformation {
    pub reference: ReferenceAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_kcal: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiber_g: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sugars_g: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub micronutrients: Vec<Micronutrient>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReferenceAmount {
    pub quantity: f64,
    pub unit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Micronutrient {
    pub nutrient_id: i64,
    pub amount: f64,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>, // enriched on output
}

/// Input shape for a single micronutrient when using --json-file (or future structured input).
/// Uses human name instead of nutrient_id; the implementation resolves/creates the nutrient.
#[derive(Deserialize, Debug, Clone)]
pub struct MicronutrientInput {
    pub name: String,
    pub amount: f64,
    pub unit: String,
}

/// Full nutrition payload accepted by `product nutrition set --json-file`.
/// Mirrors the settable parts of NutritionalInformation but is name-based for micros
/// and only derives Deserialize (it is an input type).
#[derive(Deserialize, Debug, Clone)]
pub struct NutritionInput {
    pub reference: ReferenceAmount,
    pub energy_kcal: Option<f64>,
    pub protein_g: Option<f64>,
    pub carbohydrates_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub sugars_g: Option<f64>,
    #[serde(default)]
    pub micronutrients: Vec<MicronutrientInput>,
}

// ---------- Nutrient ----------

#[derive(Serialize, Debug)]
pub struct Nutrient {
    pub id: i64,
    pub name: String,
    pub unit: String,
    pub recommended_intake: Option<f64>,
    pub created_at: TimestampInfo,
}

// ---------- Tag (product or store) ----------

#[derive(Serialize, Debug)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub created_at: TimestampInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_count: Option<i64>,
}

// ---------- Store ----------

#[derive(Serialize, Debug)]
pub struct Store {
    pub id: i64,
    pub name: String,
    pub tags: Vec<String>,
    pub created_at: TimestampInfo,
}

// ---------- Purchase ----------

#[derive(Serialize, Debug)]
pub struct Purchase {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub quantity: f64,
    pub price_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>, // human "$19.99"
    pub store_id: Option<i64>,
    pub store_name: Option<String>,
    pub purchased_at: TimestampInfo,
    pub created_at: TimestampInfo,
}

// ---------- Consumption ----------

#[derive(Serialize, Debug)]
pub struct Consumption {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub quantity: f64,
    pub unit: Option<String>,
    pub consumed_at: TimestampInfo,
    pub created_at: TimestampInfo,
}

// ---------- Report outputs ----------

#[derive(Serialize, Debug)]
pub struct NutritionReport {
    pub period: Period,
    pub total_consumed_items: i64,
    pub totals: MacroTotals,
    pub micronutrients: Vec<MicroTotal>,
}

#[derive(Serialize, Debug)]
pub struct Period {
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Serialize, Debug, Default)]
pub struct MacroTotals {
    pub energy_kcal: Option<f64>,
    pub protein_g: Option<f64>,
    pub carbohydrates_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub sugars_g: Option<f64>,
}

#[derive(Serialize, Debug)]
pub struct MicroTotal {
    pub nutrient_id: i64,
    pub name: String,
    pub unit: String,
    pub total_amount: f64,
}

#[derive(Serialize, Debug)]
pub struct SpendingReport {
    pub period: Period,
    pub total_cents: i64,
    pub total: String, // human
    pub by_store: Vec<StoreSpending>,
    pub by_product: Option<Vec<ProductSpending>>,
}

#[derive(Serialize, Debug)]
pub struct StoreSpending {
    pub store_id: Option<i64>,
    pub store_name: String,
    pub cents: i64,
    pub amount: String,
    pub purchase_count: i64,
}

#[derive(Serialize, Debug)]
pub struct ProductSpending {
    pub product_id: i64,
    pub product_name: String,
    pub cents: i64,
    pub amount: String,
    pub purchase_count: i64,
}
