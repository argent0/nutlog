use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum NutlogError {
    #[error("product not found: {0}")]
    ProductNotFound(i64),

    #[error("nutrient not found: {0}")]
    NutrientNotFound(i64),

    #[error("product tag not found: {0}")]
    ProductTagNotFound(i64),

    #[error("store not found: {0}")]
    StoreNotFound(i64),

    #[error("store tag not found: {0}")]
    StoreTagNotFound(i64),

    #[error("purchase not found: {0}")]
    PurchaseNotFound(i64),

    #[error("consumption not found: {0}")]
    ConsumptionNotFound(i64),

    #[error("product {0} has associated purchases; use --force to delete anyway")]
    ProductHasPurchases(i64),

    #[error("invalid price: {0}")]
    InvalidPrice(String),

    #[error("invalid date: {0}")]
    InvalidDate(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NutlogError>;
