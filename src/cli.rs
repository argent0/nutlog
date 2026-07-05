use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// Product ID to set (or replace) nutrition for.
    pub id: i64,

    /// Reference quantity for the nutrition values (e.g. 100 for per 100 g, 1 for per capsule/serving).
    /// Required unless --json-file is used (the payload inside the file must contain the reference).
    #[arg(long, value_name = "QTY")]
    pub reference_quantity: Option<f64>,

    /// Reference unit (e.g. g, ml, capsule, tablet, serving, piece).
    #[arg(long, value_name = "UNIT")]
    pub reference_unit: Option<String>,

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

    /// Micronutrient (or active compound). Repeat the flag for multiple.
    /// Provide three values after the flag: NAME AMOUNT UNIT.
    /// Examples:
    ///   --micronutrient "Omega 3 EPA" 181 mg
    ///   --micronutrient "Creatine Monohydrate" 5 g
    ///   --micronutrient "Magnesium elemental" 200 mg
    #[arg(
        long,
        value_names = ["NAME", "AMOUNT", "UNIT"],
        num_args = 3,
        action = clap::ArgAction::Append
    )]
    pub micronutrient: Vec<String>,

    /// Load the complete nutrition payload (reference, macros, and micronutrients) from a JSON file.
    /// The file shape uses a "reference" object and a "micronutrients" array of {name, amount, unit} objects.
    /// When --json-file is supplied, other nutrition flags are ignored and the file is authoritative.
    #[arg(long, value_name = "FILE")]
    pub json_file: Option<String>,
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
    /// Delete a nutrient. Fails if products reference it unless --force.
    Delete {
        /// Nutrient ID
        id: i64,
        /// Force delete even if product micronutrient rows reference it.
        #[arg(long)]
        force: bool,
    },
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
    /// Delete a purchase record.
    Delete { id: i64 },
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
    /// Delete a consumption record.
    Delete { id: i64 },
}

// ---------- Report ----------

/// Which macro nutrient(s) to show in per-day nutrition list output.
#[derive(Clone, Copy, Debug, ValueEnum, Eq, PartialEq)]
pub enum NutritionReportValue {
    /// All tracked macros (energy, protein, carbohydrates, fat, fiber, sugars).
    Macronutrients,
    /// Energy only (kcal).
    Calories,
    /// Protein only (g).
    Protein,
    /// Carbohydrates only (g).
    Carbohydrates,
    /// Fat only (g).
    Fat,
    /// Fiber only (g).
    Fiber,
    /// Sugars only (g).
    Sugars,
}

impl NutritionReportValue {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Macronutrients => "macronutrients",
            Self::Calories => "calories",
            Self::Protein => "protein",
            Self::Carbohydrates => "carbohydrates",
            Self::Fat => "fat",
            Self::Fiber => "fiber",
            Self::Sugars => "sugars",
        }
    }
}

/// Date range flags shared by nutrition report subcommands.
#[derive(Args, Debug, Clone)]
pub struct NutritionPeriodArgs {
    /// Start of period (inclusive). Flexible: today, yesterday, 2026-05-01, etc.
    #[arg(long)]
    pub since: Option<String>,
    /// End of period (inclusive). Same flexible date formats as --since.
    #[arg(long)]
    pub until: Option<String>,
    /// Last N calendar days inclusive of today. Cannot be combined with --since/--until.
    #[arg(long, conflicts_with_all = ["since", "until"])]
    pub days: Option<u32>,
}

#[derive(Subcommand, Debug)]
pub enum NutritionReportAction {
    /// Aggregate nutrition totals for a period.
    Summary(NutritionPeriodArgs),
    /// Per-day nutrition breakdown.
    List(NutritionListArgs),
}

#[derive(Args, Debug)]
pub struct NutritionListArgs {
    #[command(flatten)]
    pub period: NutritionPeriodArgs,
    /// Which macro value(s) to show per day.
    #[arg(long, value_enum, default_value_t = NutritionReportValue::Macronutrients)]
    pub value: NutritionReportValue,
}

#[derive(Subcommand, Debug)]
pub enum ReportAction {
    /// Nutrition intake reports (summary totals or per-day list).
    Nutrition {
        #[command(subcommand)]
        action: NutritionReportAction,
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
