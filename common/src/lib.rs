//! Types and pure logic shared by the Worker API and the Dioxus client.

use serde::{Deserialize, Serialize};

pub mod nutrition;
pub mod session;

pub use nutrition::{calc_targets, Targets};
pub use session::{sign_session, verify_session};

#[derive(Deserialize)]
#[serde(untagged)]
enum Boolish {
    Bool(bool),
    Int(i64),
}

/// D1 gives us 0/1 integers; the JSON API gives real booleans.
fn de_int_bool<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match Boolish::deserialize(d)? {
        Boolish::Bool(b) => b,
        Boolish::Int(i) => i != 0,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub sex: String,
    pub birth_year: Option<i64>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub activity: String,
    pub goal: String,
    pub calorie_target: i64,
    pub protein_target: i64,
    pub carb_target: i64,
    pub fat_target: i64,
    #[serde(deserialize_with = "de_int_bool")]
    pub share_diary: bool,
    pub household_id: Option<String>,
    #[serde(deserialize_with = "de_int_bool")]
    pub onboarded: bool,
}

impl User {
    pub fn targets(&self) -> Targets {
        Targets {
            calorie: self.calorie_target,
            protein: self.protein_target,
            carbs: self.carb_target,
            fat: self.fat_target,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Totals {
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
}

impl Totals {
    pub fn add(&mut self, o: &Totals) {
        self.kcal += o.kcal;
        self.protein += o.protein;
        self.carbs += o.carbs;
        self.fat += o.fat;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiaryEntry {
    pub id: String,
    pub log_date: String,
    pub meal: String,
    pub food_id: Option<String>,
    pub name: String,
    pub grams: f64,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub scan_item: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryResponse {
    pub date: String,
    pub targets: Targets,
    pub entries: Vec<DiaryEntry>,
    pub totals: Totals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Food {
    pub id: String,
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub serving_name: Option<String>,
    pub serving_grams: Option<f64>,
    pub kcal100: f64,
    pub protein100: f64,
    pub carbs100: f64,
    pub fat100: f64,
    #[serde(deserialize_with = "de_int_bool")]
    pub is_public: bool,
    #[serde(deserialize_with = "de_int_bool")]
    pub shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Household {
    pub id: String,
    pub name: String,
    pub invite_code: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FamilyMember {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub share_diary: bool,
    pub calorie_target: i64,
    pub kcal: f64,
    pub protein: f64,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyResponse {
    pub household: Option<Household>,
    pub members: Vec<FamilyMember>,
    pub is_owner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberDay {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub date: String,
    pub calorie_target: i64,
    pub totals: Totals,
    pub entries: Vec<DiaryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanItem {
    pub name: String,
    pub grams: f64,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    #[serde(default)]
    pub scan_id: Option<String>,
    pub name: String,
    pub description: String,
    pub items: Vec<ScanItem>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredScan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub confidence: f64,
    pub items: Vec<ScanItem>,
    pub kcal: f64,
    pub image_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineInput {
    pub scan_id: Option<String>,
    pub prompt: String,
    pub result: Option<ScanResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineResponse {
    pub valid: bool,
    pub reason: String,
    pub result: ScanResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayInsight {
    pub date: String,
    pub kcal: f64,
    pub protein: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightsResponse {
    pub days: Vec<DayInsight>,
    pub target: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub demo: bool,
    pub google: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryInput {
    pub date: String,
    pub meal: String,
    pub food_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub grams: f64,
    #[serde(default)]
    pub kcal: f64,
    #[serde(default)]
    pub protein: f64,
    #[serde(default)]
    pub carbs: f64,
    #[serde(default)]
    pub fat: f64,
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub scan_item: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInput {
    pub name: Option<String>,
    pub sex: Option<String>,
    pub birth_year: Option<i64>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub activity: Option<String>,
    pub goal: Option<String>,
    pub calorie_target: Option<i64>,
    pub share_diary: Option<bool>,
    pub units: Option<String>,
    pub onboarded: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodInput {
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub serving_name: Option<String>,
    pub serving_grams: Option<f64>,
    pub kcal100: f64,
    pub protein100: f64,
    pub carbs100: f64,
    pub fat100: f64,
    #[serde(default)]
    pub share_with_family: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInput {
    pub image_base64: String,
    pub mime_type: String,
}

pub fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Nutrients for `grams` of a food described per 100 g.
pub fn nutrients_for(kcal100: f64, p100: f64, c100: f64, f100: f64, grams: f64) -> Totals {
    let k = grams / 100.0;
    Totals {
        kcal: round1(kcal100 * k),
        protein: round1(p100 * k),
        carbs: round1(c100 * k),
        fat: round1(f100 * k),
    }
}
