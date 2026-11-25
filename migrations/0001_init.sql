PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  google_sub TEXT UNIQUE,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  picture TEXT,
  sex TEXT NOT NULL DEFAULT 'female',
  birth_year INTEGER,
  height_cm REAL,
  weight_kg REAL,
  activity TEXT NOT NULL DEFAULT 'moderate',
  goal TEXT NOT NULL DEFAULT 'lose',
  calorie_target INTEGER NOT NULL DEFAULT 1900,
  protein_target INTEGER NOT NULL DEFAULT 110,
  carb_target INTEGER NOT NULL DEFAULT 190,
  fat_target INTEGER NOT NULL DEFAULT 60,
  share_diary INTEGER NOT NULL DEFAULT 1,
  household_id TEXT,
  onboarded INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS households (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  invite_code TEXT NOT NULL UNIQUE,
  owner_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS foods (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT,
  household_id TEXT,
  name TEXT NOT NULL,
  brand TEXT,
  barcode TEXT,
  serving_name TEXT,
  serving_grams REAL,
  kcal100 REAL NOT NULL,
  protein100 REAL NOT NULL,
  carbs100 REAL NOT NULL,
  fat100 REAL NOT NULL,
  is_public INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS diary_entries (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  log_date TEXT NOT NULL,
  meal TEXT NOT NULL,
  food_id TEXT,
  name TEXT NOT NULL,
  grams REAL NOT NULL,
  kcal REAL NOT NULL,
  protein REAL NOT NULL,
  carbs REAL NOT NULL,
  fat REAL NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entries_user_date ON diary_entries (user_id, log_date);
CREATE INDEX IF NOT EXISTS idx_foods_name ON foods (name);
CREATE INDEX IF NOT EXISTS idx_foods_barcode ON foods (barcode);
CREATE INDEX IF NOT EXISTS idx_users_household ON users (household_id);
