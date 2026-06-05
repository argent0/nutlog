use clap::{Args, Parser, Subcommand};

/// nutlog - local CLI for logging food purchases, nutrition, and reports.
/// LLM-agent friendly with --json output.
#[derive(Parser, Debug)]
#[command(name = "nutlog", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output structured JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Override default SQLite database location (XDG data dir).
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<String>,

    /// Minimal output (useful for scripting/LLMs).
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage products (food items you buy).
    Product {
        #[command(subcommand)]
        action: ProductAction,
    },
    /// Manage the master list of nutrients.
    Nutrient {
        #[command(subcommand)]
        action: NutrientAction,
    },
    /// Manage product tags taxonomy.
    #[command(name = "product-tag")]
    ProductTag {
        #[command(subcommand)]
        action: ProductTagAction,
    },
    /// Record and query purchases.
    Purchase {
        #[command(subcommand)]
        action: PurchaseAction,
    },
    /// Manage stores (where you shop).
    Store {
        #[command(subcommand)]
        action: StoreAction,
    },
    /// Manage store tags taxonomy.
    #[command(name = "store-tag")]
    StoreTag {
        #[command(subcommand)]
        action: StoreTagAction,
    },
    /// Log actual consumption (what was eaten).
    Consumption {
        #[command(subcommand)]
        action: ConsumptionAction,
    },
    /// Generate reports (nutrition, spending).
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },
}

// ---------- Product ----------

#[derive(Subcommand, Debug)]
pub enum ProductAction {
    /// Create a new product.
    Create {
        /// Product name (e.g. "YOUGURISIMO 300G NATU")
        name: String,
        /// Comma-separated tags to attach on creation.
        #[arg(long, value_name = "TAGS", value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
    /// List all products (newest first).
    List,
    /// Fuzzy search products by name or tag.
    Search {
        /// Search term for name (fuzzy).
        #[arg(long)]
        name: Option<String>,
        /// Filter by exact tag name.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show full details for a product (incl. tags, nutrition).
    Show {
        /// Product ID
        id: i64,
    },
    /// Rename a product.
    Rename {
        /// Product ID
        id: i64,
        /// New name
        #[arg(long, value_name = "NAME")]
        name: String,
    },
    /// Add or remove tags on a product.
    Tag {
        #[command(subcommand)]
        action: ProductTagModifyAction,
    },
    /// Delete a product. Fails if purchases exist unless --force.
    Delete {
        /// Product ID
        id: i64,
        /// Force delete even if purchases/consumptions reference it.
        #[arg(long)]
        force: bool,
    },
    /// Set or update nutritional information for a product.
    Nutrition {
        #[command(subcommand)]
        action: NutritionAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProductTagModifyAction {
    /// Attach a tag to product (creates tag if missing).
    Add {
        id: i64,
        #[arg(long)]
        tag: String,
    },
    /// Detach a tag from product.
    Remove {
        id: i64,
        #[arg(long)]
        tag: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum NutritionAction {
    /// Set / replace nutrition facts for the product.
    Set(NutritionSetArgs),
}

#[derive(Args, Debug)]
pub struct NutritionSetArgs {
    pub id: i64,
    #[arg(long, value_name = "QTY")]
    pub reference_quantity: f64,
    #[arg(long, value_name = "UNIT")]
    pub reference_unit: String,
    #[arg(long)]
    pub energy_kcal: Option<f64>,
    #[arg(long)]
    pub protein_g: Option<f64>,
    #[arg(long)]
    pub carbohydrates_g: Option<f64>,
    #[arg(long)]
    pub fat_g: Option<f64>,
    #[arg(long)]
    pub fiber_g: Option<f64>,
    #[arg(long)]
    pub sugars_g: Option<f64>,
    // Micronutrients not exposed in simple CLI for set (use JSON? or extend later).
    // For now, basic macros via flags; micros via future or direct DB ok for agent.
}

// ---------- Nutrient ----------

#[derive(Subcommand, Debug)]
pub enum NutrientAction {
    /// List all nutrients (pre-populated + custom).
    List,
    /// Create a custom nutrient.
    Create {
        name: String,
        #[arg(long)]
        unit: String,
        #[arg(long, value_name = "AMOUNT")]
        recommended_intake: Option<f64>,
    },
    /// Show a nutrient.
    Show { id: i64 },
    /// Fuzzy search nutrients by name.
    Search { query: String },
}

// ---------- Product Tag ----------

#[derive(Subcommand, Debug)]
pub enum ProductTagAction {
    /// Create a new product tag.
    Create { name: String },
    /// List all product tags.
    List,
    /// Fuzzy search product tags.
    Search { query: String },
    /// Show tag details (and products using it).
    Show { id: i64 },
    /// Delete a product tag (removes associations).
    Delete { id: i64 },
}

// ---------- Purchase ----------

#[derive(Subcommand, Debug)]
pub enum PurchaseAction {
    /// Record a purchase of a product.
    Create {
        /// Product ID
        product_id: i64,
        /// Price, e.g. 4.99 or $19.99 (stored in cents)
        #[arg(long)]
        price: Option<String>,
        /// Store ID (optional)
        #[arg(long)]
        store: Option<i64>,
        /// Date of purchase (today, yesterday, 2026-05-20, last week, etc.)
        #[arg(long, default_value = "today")]
        date: String,
        /// Quantity purchased (default 1)
        #[arg(long, default_value_t = 1.0)]
        quantity: f64,
    },
    /// List purchases (optionally filtered).
    List {
        /// Only since this date (inclusive)
        #[arg(long)]
        since: Option<String>,
        /// Only until this date (inclusive)
        #[arg(long)]
        until: Option<String>,
        /// Filter by product
        #[arg(long)]
        product: Option<i64>,
        /// Filter by store
        #[arg(long)]
        store: Option<i64>,
    },
    /// Show a single purchase.
    Show { id: i64 },
}

// ---------- Store ----------

#[derive(Subcommand, Debug)]
pub enum StoreAction {
    Create {
        name: String,
    },
    List,
    Show {
        id: i64,
    },
    Rename {
        id: i64,
        #[arg(long)]
        name: String,
    },
    Tag {
        #[command(subcommand)]
        action: StoreTagModifyAction,
    },
    Delete {
        id: i64,
    },
}

#[derive(Subcommand, Debug)]
pub enum StoreTagModifyAction {
    Add {
        id: i64,
        #[arg(long)]
        tag: String,
    },
    Remove {
        id: i64,
        #[arg(long)]
        tag: String,
    },
}

// ---------- Store Tag ----------

#[derive(Subcommand, Debug)]
pub enum StoreTagAction {
    Create { name: String },
    List,
    Search { query: String },
    Show { id: i64 },
    Delete { id: i64 },
}

// ---------- Consumption ----------

#[derive(Subcommand, Debug)]
pub enum ConsumptionAction {
    /// Record consumption (what was actually eaten/drunk).
    Create {
        product_id: i64,
        /// Amount consumed. If omitted, tool may hint reference amount.
        #[arg(long)]
        quantity: Option<f64>,
        /// Unit for quantity, e.g. g, ml. If omitted with quantity, assumes g?
        #[arg(long)]
        unit: Option<String>,
        #[arg(long, default_value = "today")]
        date: String,
    },
    /// List consumption records.
    List {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        product: Option<i64>,
    },
}

// ---------- Report ----------

#[derive(Subcommand, Debug)]
pub enum ReportAction {
    /// Nutrition intake summary based on consumption in period.
    Nutrition {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
    },
    /// Spending summary.
    Spending {
        /// Group by: total, store, product, month etc.
        #[arg(long, default_value = "total")]
        by: String,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        /// e.g. month, year for period grouping
        #[arg(long)]
        period: Option<String>,
    },
}
