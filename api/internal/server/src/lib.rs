use mycals_common::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::*;

const SESSION_COOKIE: &str = "mycals_session";
const SESSION_DAYS: i64 = 30;

pub async fn handle(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let mut res = match dispatch(req, env).await {
        Ok(r) => r,
        Err(e) => err(500, &format!("server error: {e}")),
    };
    {
        let h = res.headers_mut();
        let _ = h.set("access-control-allow-origin", "*");
        let _ = h.set("access-control-allow-headers", "content-type, authorization");
        let _ = h.set("access-control-allow-methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS");
        let _ = h.set("cache-control", "no-store");
    }
    Ok(res)
}

async fn dispatch(mut req: Request, env: Env) -> Result<Response> {
    let method = req.method().as_ref().to_string();
    let url = req.url()?;
    let path = url.path().to_string();

    if method == "OPTIONS" {
        return Ok(Response::empty()?.with_status(204));
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/api/health") => return Ok(json_msg("ok", 200)),
        ("GET", "/api/config") => {
            let demo = var_str(&env, "DEMO_MODE") == "1";
            let google = env.secret("GOOGLE_CLIENT_ID").is_ok();
            return json(&ConfigResponse { demo, google }, 200);
        }
        ("GET", "/api/auth/google/start") => return google_start(&req, &env).await,
        ("GET", "/api/auth/google/callback") => return google_callback(&req, &env).await,
        ("POST", "/api/dev/login") => return demo_login(&env).await,
        ("POST", "/api/auth/mobile/exchange") => return mobile_exchange(&mut req, &env).await,
        ("POST", "/api/logout") => {
            let mut res = json_msg("signed out", 200);
            let _ = set_cookie(&mut res, SESSION_COOKIE, "", 0, false);
            return Ok(res);
        }
        ("GET", "/api/me") => {
            let Some(user) = current_user(&req, &env).await? else {
                return Ok(err(401, "Not signed in"));
            };
            return json(&user, 200);
        }
        ("PUT", "/api/profile") => return update_profile(&mut req, &env).await,
        ("GET", "/api/diary") => return get_diary(&req, &env).await,
        ("POST", "/api/diary/entries") => return add_entry(&mut req, &env).await,
        ("GET", "/api/foods") => return search_foods(&req, &env).await,
        ("POST", "/api/foods") => return create_food(&mut req, &env).await,
        ("POST", "/api/scan") => return scan_food(&mut req, &env).await,
        ("GET", "/api/family") => return family_get(&req, &env).await,
        ("POST", "/api/family") => return family_create(&mut req, &env).await,
        ("POST", "/api/family/join") => return family_join(&mut req, &env).await,
        ("POST", "/api/family/leave") => return family_leave(&req, &env).await,
        ("POST", "/api/family/rotate") => return family_rotate(&req, &env).await,
        ("GET", "/api/insights") => return insights(&req, &env).await,
        _ => {}
    }

    if method == "PATCH" && path.starts_with("/api/diary/entries/") {
        let id = path.trim_start_matches("/api/diary/entries/").to_string();
        return patch_entry(&mut req, &env, &id).await;
    }
    if method == "DELETE" && path.starts_with("/api/diary/entries/") {
        let id = path.trim_start_matches("/api/diary/entries/").to_string();
        return delete_entry(&req, &env, &id).await;
    }
    if method == "GET" && path.starts_with("/api/foods/barcode/") {
        let code = path.trim_start_matches("/api/foods/barcode/").to_string();
        return barcode_lookup(&env, &code).await;
    }
    if method == "GET" && path.starts_with("/api/family/members/") && path.ends_with("/diary") {
        let id = path
            .trim_start_matches("/api/family/members/")
            .trim_end_matches("/diary")
            .trim_end_matches('/')
            .to_string();
        return member_diary(&req, &env, &id).await;
    }

    Ok(err(404, "Not found"))
}

// ---------------------------------------------------------------- auth

async fn google_start(req: &Request, env: &Env) -> Result<Response> {
    let Ok(client_id) = env.secret("GOOGLE_CLIENT_ID") else {
        return Ok(err(
            503,
            "Google sign-in is not configured. Add GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET as Worker secrets.",
        ));
    };
    let url = req.url()?;
    let mobile = query(req, "mobile").as_deref() == Some("1");
    let next = query(req, "next")
        .filter(|n| n.starts_with('/') && !n.starts_with("//"))
        .unwrap_or_else(|| "/".into());
    let state = new_id("st");
    let redirect = format!("{}/api/auth/google/callback", origin(&url));
    let mut g = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
    {
        let mut q = g.query_pairs_mut();
        q.append_pair("client_id", client_id.to_string().as_str());
        q.append_pair("redirect_uri", &redirect);
        q.append_pair("response_type", "code");
        q.append_pair("scope", "openid email profile");
        q.append_pair("state", &state);
        q.append_pair("prompt", "select_account");
    }
    let mut res = redirect_to(g.as_str())?;
    let secure = url.scheme() == "https";
    set_cookie(&mut res, "oauth_state", &state, 600, secure)?;
    set_cookie(
        &mut res,
        "oauth_next",
        if mobile { "m" } else { &next },
        600,
        secure,
    )?;
    Ok(res)
}

async fn google_callback(req: &Request, env: &Env) -> Result<Response> {
    let Some(state) = query(req, "state") else {
        return Ok(err(400, "Missing state"));
    };
    let Some(code) = query(req, "code") else {
        return Ok(err(400, "Google sign-in was cancelled"));
    };
    if cookie(req, "oauth_state").as_deref() != Some(state.as_str()) {
        return Ok(err(400, "Sign-in state expired. Please try again."));
    }
    let next = cookie(req, "oauth_next").unwrap_or_else(|| "/".into());
    let (Ok(client_id), Ok(client_secret)) = (
        env.secret("GOOGLE_CLIENT_ID"),
        env.secret("GOOGLE_CLIENT_SECRET"),
    ) else {
        return Ok(err(503, "Google sign-in is not configured"));
    };
    let url = req.url()?;
    let redirect_uri = format!("{}/api/auth/google/callback", origin(&url));
    let form = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("code", &code)
        .append_pair("client_id", &client_id.to_string())
        .append_pair("client_secret", &client_secret.to_string())
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("grant_type", "authorization_code")
        .finish();

    let token_json = post_text(
        "https://oauth2.googleapis.com/token",
        &form,
        "application/x-www-form-urlencoded",
        None,
    )
    .await?;
    let token: Value = serde_json::from_str(&token_json).unwrap_or_default();
    let Some(access) = token["access_token"].as_str() else {
        return Ok(err(401, "Google token exchange failed"));
    };
    let userinfo = get_text(
        "https://www.googleapis.com/oauth2/v3/userinfo",
        Some(access),
    )
    .await?;
    let info: Value = serde_json::from_str(&userinfo).unwrap_or_default();
    let Some(sub) = info["sub"].as_str() else {
        return Ok(err(401, "Google did not return a user profile"));
    };
    if info["email_verified"].as_bool() == Some(false) {
        return Ok(err(401, "Your Google email is not verified"));
    }
    let email = info["email"].as_str().unwrap_or_default().to_string();
    let name = info["name"].as_str().unwrap_or_else(|| {
        email.split('@').next().unwrap_or("Friend")
    });
    let picture = info["picture"].as_str();
    let db = env.d1("DB")?;
    let user = upsert_google_user(&db, sub, &email, name, picture).await?;
    let token = sign_session(&session_secret(env)?, &user.id, now(), SESSION_DAYS * 86_400);

    if next == "m" {
        let code = new_id("mob");
        exec(
            &db,
            "INSERT INTO mobile_auth_codes (code, user_id, expires_at) VALUES (?1, ?2, ?3)",
            vec![js(&code), js(&user.id), jn((now() + 300) as f64)],
        )
        .await?;
        let mut res = redirect_to(&format!("mycals://auth?code={code}"))?;
        set_cookie(&mut res, SESSION_COOKIE, &token, SESSION_DAYS * 86_400, true)?;
        return Ok(res);
    }
    let mut res = redirect_to(&format!("{}{}", origin(&url), next))?;
    set_cookie(
        &mut res,
        SESSION_COOKIE,
        &token,
        SESSION_DAYS * 86_400,
        url.scheme() == "https",
    )?;
    Ok(res)
}

async fn upsert_google_user(
    db: &D1Database,
    sub: &str,
    email: &str,
    name: &str,
    picture: Option<&str>,
) -> Result<User> {
    let existing: Option<User> = one(
        db,
        "SELECT * FROM users WHERE google_sub = ?1",
        vec![js(sub)],
    )
    .await?;
    if let Some(u) = existing {
        exec(
            db,
            "UPDATE users SET email = ?1, name = ?2, picture = ?3, updated_at = datetime('now') WHERE id = ?4",
            vec![js(email), js(name), picture.map(js).unwrap_or(JsValue::NULL), js(&u.id)],
        )
        .await?;
        return one(db, "SELECT * FROM users WHERE id = ?1", vec![js(&u.id)])
            .await?
            .ok_or_else(|| Error::Internal("user vanished".into()));
    }
    let id = new_id("usr");
    exec(
        db,
        "INSERT INTO users (id, google_sub, email, name, picture, onboarded) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        vec![
            js(&id),
            js(sub),
            js(email),
            js(name),
            picture.map(js).unwrap_or(JsValue::NULL),
        ],
    )
    .await?;
    one(db, "SELECT * FROM users WHERE id = ?1", vec![js(&id)])
        .await?
        .ok_or_else(|| Error::Internal("user insert failed".into()))
}

async fn mobile_exchange(req: &mut Request, env: &Env) -> Result<Response> {
    #[derive(Deserialize)]
    struct Body {
        code: String,
    }
    #[derive(Deserialize)]
    struct CodeRow {
        user_id: String,
        expires_at: f64,
        used: i64,
    }
    let body: Body = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid sign-in code")),
    };
    let code = body.code.trim().to_string();
    if code.is_empty() {
        return Ok(err(400, "Invalid sign-in code"));
    }
    let db = env.d1("DB")?;
    let row: Option<CodeRow> = one(
        &db,
        "SELECT user_id, expires_at, used FROM mobile_auth_codes WHERE code=?1",
        vec![js(&code)],
    )
    .await?;
    let Some(row) = row else {
        return Ok(err(401, "Sign-in code expired. Try again."));
    };
    if row.used != 0 || row.expires_at < now() as f64 {
        return Ok(err(401, "Sign-in code expired. Try again."));
    }
    exec(
        &db,
        "UPDATE mobile_auth_codes SET used=1 WHERE code=?1",
        vec![js(&code)],
    )
    .await?;
    let user: Option<User> = one(&db, "SELECT * FROM users WHERE id=?1", vec![js(&row.user_id)]).await?;
    let Some(user) = user else {
        return Ok(err(401, "Sign-in failed. Try again."));
    };
    let token = sign_session(&session_secret(env)?, &user.id, now(), SESSION_DAYS * 86_400);
    json(&AuthResponse { token, user }, 200)
}

async fn demo_login(env: &Env) -> Result<Response> {
    if var_str(env, "DEMO_MODE") != "1" {
        return Ok(err(404, "Demo mode is off"));
    }
    let db = env.d1("DB")?;
    seed_demo(&db).await?;
    let user: User = one(&db, "SELECT * FROM users WHERE id = 'usr_demo'", vec![])
        .await?
        .ok_or_else(|| Error::Internal("demo seed failed".into()))?;
    let token = sign_session(&session_secret(env)?, &user.id, now(), SESSION_DAYS * 86_400);
    let mut res = json(&AuthResponse { token: token.clone(), user }, 200)?;
    set_cookie(&mut res, SESSION_COOKIE, &token, SESSION_DAYS * 86_400, false)?;
    Ok(res)
}

async fn seed_demo(db: &D1Database) -> Result<()> {
    exec(
        db,
        "INSERT OR IGNORE INTO households (id, name, invite_code, owner_id) VALUES ('hh_demo', 'The Demo Family', 'DEMO42', 'usr_demo')",
        vec![],
    )
    .await?;
    exec(
        db,
        "INSERT OR IGNORE INTO users (id, email, name, sex, birth_year, height_cm, weight_kg, activity, goal, calorie_target, protein_target, carb_target, fat_target, share_diary, household_id, onboarded)
         VALUES ('usr_demo', 'demo@mycals.app', 'Alex', 'female', 1994, 168, 63, 'moderate', 'lose', 1750, 126, 165, 55, 1, 'hh_demo', 1)",
        vec![],
    )
    .await?;
    exec(
        db,
        "INSERT OR IGNORE INTO users (id, email, name, sex, birth_year, height_cm, weight_kg, activity, goal, calorie_target, protein_target, carb_target, fat_target, share_diary, household_id, onboarded)
         VALUES ('usr_maya', 'maya@mycals.app', 'Maya', 'female', 1996, 163, 57, 'light', 'maintain', 1850, 100, 210, 55, 1, 'hh_demo', 1)",
        vec![],
    )
    .await?;
    exec(
        db,
        "INSERT OR IGNORE INTO users (id, email, name, sex, birth_year, height_cm, weight_kg, activity, goal, calorie_target, protein_target, carb_target, fat_target, share_diary, household_id, onboarded)
         VALUES ('usr_sam', 'sam@mycals.app', 'Sam', 'male', 1990, 181, 79, 'active', 'gain', 2600, 140, 300, 80, 1, 'hh_demo', 1)",
        vec![],
    )
    .await?;
    let rows = [
        ("usr_demo", "breakfast", "Greek yogurt, berries & honey", 320.0, 22.0, 38.0, 7.0),
        ("usr_demo", "lunch", "Chicken rice bowl", 560.0, 42.0, 62.0, 14.0),
        ("usr_demo", "snacks", "Almonds", 170.0, 6.0, 6.0, 15.0),
        ("usr_maya", "breakfast", "Oatmeal with banana", 310.0, 9.0, 54.0, 7.0),
        ("usr_maya", "lunch", "Paneer salad wrap", 480.0, 24.0, 45.0, 22.0),
        ("usr_sam", "breakfast", "Eggs on toast", 420.0, 26.0, 30.0, 21.0),
        ("usr_sam", "lunch", "Chicken biryani", 680.0, 34.0, 78.0, 24.0),
        ("usr_sam", "snacks", "Protein shake", 190.0, 30.0, 8.0, 4.0),
    ];
    for (i, (uid, meal, name, kcal, p, c, f)) in rows.iter().enumerate() {
        // Seed yesterday/today/tomorrow so the demo looks right in every client timezone.
        for offset in ["-1 day", "0 day", "+1 day"] {
            exec(
                db,
                "INSERT OR IGNORE INTO diary_entries (id, user_id, log_date, meal, name, grams, kcal, protein, carbs, fat)
                 VALUES (?1 || '_' || date('now', ?9), ?2, date('now', ?9), ?3, ?4, 100, ?5, ?6, ?7, ?8)",
                vec![
                    js(&format!("demo_{i}_")),
                    js(uid),
                    js(meal),
                    js(name),
                    jn(*kcal),
                    jn(*p),
                    jn(*c),
                    jn(*f),
                    js(offset),
                ],
            )
            .await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- profile

async fn update_profile(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let input: ProfileInput = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid profile")),
    };
    let name = input.name.filter(|s| !s.trim().is_empty()).unwrap_or(user.name);
    let sex = input.sex.unwrap_or(user.sex);
    let birth_year = input.birth_year.or(user.birth_year).unwrap_or(1995);
    let height = input.height_cm.or(user.height_cm).unwrap_or(170.0);
    let weight = input.weight_kg.or(user.weight_kg).unwrap_or(70.0);
    let activity = input.activity.unwrap_or(user.activity);
    let goal = input.goal.unwrap_or(user.goal);
    let share = input.share_diary.unwrap_or(user.share_diary);
    let onboarded = input.onboarded.unwrap_or(true);
    let age = 2026 - birth_year;
    let t = calc_targets(&sex, age, height, weight, &activity, &goal);
    let db = env.d1("DB")?;
    exec(
        &db,
        "UPDATE users SET name=?1, sex=?2, birth_year=?3, height_cm=?4, weight_kg=?5,
         activity=?6, goal=?7, calorie_target=?8, protein_target=?9, carb_target=?10,
         fat_target=?11, share_diary=?12, onboarded=?13, updated_at=datetime('now') WHERE id=?14",
        vec![
            js(&name), js(&sex), jn(birth_year as f64), jn(height), jn(weight),
            js(&activity), js(&goal), jn(t.calorie as f64), jn(t.protein as f64),
            jn(t.carbs as f64), jn(t.fat as f64), jn(if share {1.0} else {0.0}),
            jn(if onboarded {1.0} else {0.0}), js(&user.id),
        ],
    )
    .await?;
    let updated: User = one(&db, "SELECT * FROM users WHERE id=?1", vec![js(&user.id)])
        .await?
        .ok_or_else(|| Error::Internal("profile update failed".into()))?;
    json(&updated, 200)
}

// ---------------------------------------------------------------- diary

async fn get_diary(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let date = query(req, "date");
    let db = env.d1("DB")?;
    let (date, entries) = day_entries(&db, &user.id, date.as_deref()).await?;
    let mut totals = Totals::default();
    for e in &entries {
        totals.add(&Totals { kcal: e.kcal, protein: e.protein, carbs: e.carbs, fat: e.fat });
    }
    let totals = Totals {
        kcal: round1(totals.kcal),
        protein: round1(totals.protein),
        carbs: round1(totals.carbs),
        fat: round1(totals.fat),
    };
    json(
        &DiaryResponse {
            date,
            targets: user.targets(),
            entries,
            totals,
        },
        200,
    )
}

async fn day_entries(
    db: &D1Database,
    user_id: &str,
    date: Option<&str>,
) -> Result<(String, Vec<DiaryEntry>)> {
    let date = match date {
        Some(d) if is_date(d) => d.to_string(),
        _ => today(db).await?,
    };
    let entries: Vec<DiaryEntry> = all(
        db,
        "SELECT id, log_date, meal, food_id, name, grams, kcal, protein, carbs, fat
         FROM diary_entries WHERE user_id=?1 AND log_date=?2 ORDER BY created_at, rowid",
        vec![js(user_id), js(&date)],
    )
    .await?;
    Ok((date, entries))
}

async fn add_entry(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let input: EntryInput = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid entry")),
    };
    if !is_date(&input.date) {
        return Ok(err(400, "Use a YYYY-MM-DD date"));
    }
    if !is_meal(&input.meal) {
        return Ok(err(400, "Unknown meal"));
    }
    let db = env.d1("DB")?;
    let (name, grams, totals) = if let Some(food_id) = input.food_id.as_deref() {
        let sql = "SELECT *, CASE WHEN household_id IS NOT NULL THEN 1 ELSE 0 END AS shared FROM foods
                   WHERE id=?1 AND (is_public=1 OR owner_user_id=?2 OR (household_id IS NOT NULL AND household_id=?3))";
        let food: Option<Food> = one(
            &db,
            sql,
            vec![
                js(food_id),
                js(&user.id),
                user.household_id.clone().map(|h| js(&h)).unwrap_or(JsValue::NULL),
            ],
        )
        .await?;
        let Some(food) = food else {
            return Ok(err(404, "Food not found"));
        };
        let grams = if input.grams > 0.0 {
            input.grams
        } else {
            food.serving_grams.unwrap_or(100.0)
        };
        let t = nutrients_for(food.kcal100, food.protein100, food.carbs100, food.fat100, grams);
        (food.name, grams, t)
    } else {
        if input.name.trim().is_empty() {
            return Ok(err(400, "Give this food a name"));
        }
        let grams = if input.grams > 0.0 { input.grams } else { 100.0 };
        (
            input.name.clone(),
            grams,
            Totals {
                kcal: input.kcal.max(0.0),
                protein: input.protein.max(0.0),
                carbs: input.carbs.max(0.0),
                fat: input.fat.max(0.0),
            },
        )
    };
    let id = new_id("ent");
    exec(
        &db,
        "INSERT INTO diary_entries (id, user_id, log_date, meal, food_id, name, grams, kcal, protein, carbs, fat)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        vec![
            js(&id), js(&user.id), js(&input.date), js(&input.meal),
            input.food_id.clone().map(|f| js(&f)).unwrap_or(JsValue::NULL),
            js(&name), jn(grams), jn(totals.kcal), jn(totals.protein),
            jn(totals.carbs), jn(totals.fat),
        ],
    )
    .await?;
    json(
        &DiaryEntry {
            id,
            log_date: input.date,
            meal: input.meal,
            food_id: input.food_id,
            name,
            grams,
            kcal: totals.kcal,
            protein: totals.protein,
            carbs: totals.carbs,
            fat: totals.fat,
        },
        201,
    )
}

async fn patch_entry(req: &mut Request, env: &Env, id: &str) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    #[derive(Deserialize)]
    struct Body { grams: f64 }
    let body: Body = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid entry")),
    };
    if !(1.0..=5000.0).contains(&body.grams) {
        return Ok(err(400, "Portion must be between 1 and 5000 g"));
    }
    let db = env.d1("DB")?;
    let entry: Option<DiaryEntry> = one(
        &db,
        "SELECT id, log_date, meal, food_id, name, grams, kcal, protein, carbs, fat
         FROM diary_entries WHERE id=?1 AND user_id=?2",
        vec![js(id), js(&user.id)],
    )
    .await?;
    let Some(mut entry) = entry else {
        return Ok(err(404, "Entry not found"));
    };
    let scale = if entry.grams > 0.0 { body.grams / entry.grams } else { 1.0 };
    entry.grams = body.grams;
    entry.kcal = round1(entry.kcal * scale);
    entry.protein = round1(entry.protein * scale);
    entry.carbs = round1(entry.carbs * scale);
    entry.fat = round1(entry.fat * scale);
    exec(
        &db,
        "UPDATE diary_entries SET grams=?1, kcal=?2, protein=?3, carbs=?4, fat=?5 WHERE id=?6",
        vec![jn(entry.grams), jn(entry.kcal), jn(entry.protein), jn(entry.carbs), jn(entry.fat), js(id)],
    )
    .await?;
    json(&entry, 200)
}

async fn delete_entry(req: &Request, env: &Env, id: &str) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let db = env.d1("DB")?;
    exec(
        &db,
        "DELETE FROM diary_entries WHERE id=?1 AND user_id=?2",
        vec![js(id), js(&user.id)],
    )
    .await?;
    Ok(json_msg("deleted", 200))
}

// ---------------------------------------------------------------- foods

async fn search_foods(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let q = query(req, "q").unwrap_or_default().to_lowercase();
    let db = env.d1("DB")?;
    let foods: Vec<Food> = all(
        &db,
        "SELECT *, CASE WHEN household_id IS NOT NULL THEN 1 ELSE 0 END AS shared FROM foods
         WHERE (is_public=1 OR owner_user_id=?1 OR (household_id IS NOT NULL AND household_id=?2))
           AND (lower(name) LIKE ?3 OR lower(COALESCE(brand,'')) LIKE ?3)
         ORDER BY is_public DESC, name LIMIT 60",
        vec![
            js(&user.id),
            user.household_id.clone().map(|h| js(&h)).unwrap_or(JsValue::NULL),
            js(&format!("%{q}%")),
        ],
    )
    .await?;
    json(&foods, 200)
}

async fn create_food(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let input: FoodInput = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid food")),
    };
    if input.name.trim().is_empty() {
        return Ok(err(400, "Give this food a name"));
    }
    if input.kcal100 < 0.0 || input.kcal100 > 900.0 {
        return Ok(err(400, "Calories per 100 g must be 0-900"));
    }
    let id = new_id("food");
    let household = if input.share_with_family {
        user.household_id.clone()
    } else {
        None
    };
    let db = env.d1("DB")?;
    let shared: f64 = if household.is_some() { 1.0 } else { 0.0 };
    exec(
        &db,
        "INSERT INTO foods (id, owner_user_id, household_id, name, brand, barcode, serving_name, serving_grams, kcal100, protein100, carbs100, fat100, is_public)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)",
        vec![
            js(&id), js(&user.id),
            household.clone().map(|h| js(&h)).unwrap_or(JsValue::NULL),
            js(input.name.trim()), input.brand.clone().map(|b| js(&b)).unwrap_or(JsValue::NULL),
            input.barcode.clone().map(|b| js(&b)).unwrap_or(JsValue::NULL),
            input.serving_name.clone().map(|s| js(&s)).unwrap_or(JsValue::NULL),
            input.serving_grams.map(jn).unwrap_or(JsValue::NULL),
            jn(input.kcal100), jn(input.protein100), jn(input.carbs100), jn(input.fat100),
        ],
    )
    .await?;
    json(
        &Food {
            id, name: input.name, brand: input.brand, barcode: input.barcode,
            serving_name: input.serving_name, serving_grams: input.serving_grams,
            kcal100: input.kcal100, protein100: input.protein100,
            carbs100: input.carbs100, fat100: input.fat100,
            is_public: false, shared: shared > 0.0,
        },
        201,
    )
}

async fn barcode_lookup(env: &Env, code: &str) -> Result<Response> {
    let db = env.d1("DB")?;
    let found: Option<Food> = one(
        &db,
        "SELECT *, CASE WHEN household_id IS NOT NULL THEN 1 ELSE 0 END AS shared FROM foods
         WHERE barcode=?1 AND is_public=1 LIMIT 1",
        vec![js(code)],
    )
    .await?;
    if let Some(f) = found {
        return json(&f, 200);
    }
    let url = format!(
        "https://world.openfoodfacts.org/api/v2/product/{code}.json?fields=code,product_name,brands,serving_size,serving_quantity,nutriments"
    );
    let body = get_text(&url, None).await?;
    let v: Value = serde_json::from_str(&body).unwrap_or_default();
    if v["status"].as_i64() != Some(1) {
        return Ok(err(404, "No food found for that barcode"));
    }
    let p = &v["product"];
    let n = &p["nutriments"];
    let per100 = |k: &str| n[k].as_f64().unwrap_or(0.0);
    let mut kcal = per100("energy-kcal_100g");
    if kcal == 0.0 {
        kcal = per100("energy_100g") / 4.184;
    }
    json(
        &Food {
            id: format!("off_{code}"),
            name: p["product_name"].as_str().unwrap_or("Scanned food").to_string(),
            brand: p["brands"].as_str().map(|s| s.to_string()),
            barcode: Some(code.to_string()),
            serving_name: p["serving_size"].as_str().map(|s| s.to_string()),
            serving_grams: p["serving_quantity"].as_f64(),
            kcal100: round1(kcal),
            protein100: round1(per100("proteins_100g")),
            carbs100: round1(per100("carbohydrates_100g")),
            fat100: round1(per100("fat_100g")),
            is_public: true,
            shared: true,
        },
        200,
    )
}

// ---------------------------------------------------------------- gemini scan

async fn scan_food(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(_user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let Ok(key) = env.secret("GEMINI_API_KEY") else {
        return Ok(err(
            503,
            "Photo scanning is not configured yet. Add GEMINI_API_KEY as a Worker secret.",
        ));
    };
    let input: ScanInput = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Invalid scan request")),
    };
    if input.image_base64.len() < 32 {
        return Ok(err(400, "No photo received"));
    }
    if input.image_base64.len() > 12_000_000 {
        return Ok(err(413, "Photo is too large (max ~8 MB)"));
    }
    let model = if var_str(env, "GEMINI_MODEL").is_empty() {
        "gemini-2.5-flash".to_string()
    } else {
        var_str(env, "GEMINI_MODEL")
    };
    let schema = json!({
        "type": "OBJECT",
        "properties": {
            "name": {"type": "STRING"},
            "description": {"type": "STRING"},
            "confidence": {"type": "NUMBER"},
            "items": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "name": {"type": "STRING"},
                        "grams": {"type": "NUMBER"},
                        "kcal": {"type": "NUMBER"},
                        "protein": {"type": "NUMBER"},
                        "carbs": {"type": "NUMBER"},
                        "fat": {"type": "NUMBER"}
                    },
                    "required": ["name", "grams", "kcal", "protein", "carbs", "fat"]
                }
            }
        },
        "required": ["name", "description", "items", "confidence"]
    });
    let body = json!({
        "contents": [{
            "parts": [
                {"text": "You are a nutrition expert. Identify the meal or food label in this photo. Break it into distinct components, estimate the portion in grams for what is shown, and estimate calories, protein, carbs and fat for that portion (not per 100 g). If a nutrition label is readable, use those numbers. Title the meal briefly. Confidence is 0 to 1."},
                {"inline_data": {"mime_type": input.mime_type, "data": input.image_base64}}
            ]
        }],
        "generationConfig": {
            "temperature": 0.2,
            "responseMimeType": "application/json",
            "responseSchema": schema
        }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
    );
    let text = post_text(&url, &body.to_string(), "application/json", None).await?;
    let v: Value = serde_json::from_str(&text).unwrap_or_default();
    let raw = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let mut result: ScanResult = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(_) => return Ok(err(502, "Gemini could not read that photo. Try a clearer shot.")),
    };
    result.items.retain(|i| i.kcal >= 0.0 && i.grams >= 0.0);
    if result.items.is_empty() {
        return Ok(err(422, "No food found in that photo. Try again."));
    }
    result.confidence = result.confidence.clamp(0.0, 1.0);
    json(&result, 200)
}

// ---------------------------------------------------------------- family

async fn family_get(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let date = query(req, "date");
    let db = env.d1("DB")?;
    family_payload(&db, &user, date.as_deref()).await
}

async fn family_payload(db: &D1Database, user: &User, date: Option<&str>) -> Result<Response> {
    let date = match date {
        Some(d) if is_date(d) => d.to_string(),
        _ => today(db).await?,
    };
    let Some(hid) = user.household_id.as_deref() else {
        return json(&FamilyResponse { household: None, members: vec![], is_owner: false }, 200);
    };
    let household: Option<Household> = one(db, "SELECT * FROM households WHERE id=?1", vec![js(hid)]).await?;
    let Some(household) = household else {
        return json(&FamilyResponse { household: None, members: vec![], is_owner: false }, 200);
    };
    let members: Vec<User> = all(db, "SELECT * FROM users WHERE household_id=?1 ORDER BY created_at", vec![js(hid)]).await?;
    let mut out = Vec::new();
    for m in members {
        let s: Option<Sum> = one(
            db,
            "SELECT COALESCE(SUM(kcal),0) AS kcal, COALESCE(SUM(protein),0) AS protein, COUNT(*) AS entry_count
             FROM diary_entries WHERE user_id=?1 AND log_date=?2",
            vec![js(&m.id), js(&date)],
        )
        .await?;
        let s = s.unwrap_or_default();
        out.push(FamilyMember {
            id: m.id.clone(),
            name: m.name.clone(),
            picture: m.picture.clone(),
            share_diary: m.share_diary,
            calorie_target: m.calorie_target,
            kcal: round1(s.kcal),
            protein: round1(s.protein),
            entry_count: s.entry_count,
        });
    }
    json(
        &FamilyResponse {
            is_owner: household.owner_id == user.id,
            household: Some(household),
            members: out,
        },
        200,
    )
}

async fn family_create(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    if user.household_id.is_some() {
        return Ok(err(400, "You are already in a family"));
    }
    #[derive(Deserialize)]
    struct Body { name: Option<String> }
    let body: Body = req.json().await.unwrap_or(Body { name: None });
    let db = env.d1("DB")?;
    let id = new_id("hh");
    let name = body.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| format!("{}'s family", user.name));
    let code = unique_code(&db).await?;
    exec(
        &db,
        "INSERT INTO households (id, name, invite_code, owner_id) VALUES (?1, ?2, ?3, ?4)",
        vec![js(&id), js(&name), js(&code), js(&user.id)],
    )
    .await?;
    exec(&db, "UPDATE users SET household_id=?1 WHERE id=?2", vec![js(&id), js(&user.id)]).await?;
    let fresh = one(&db, "SELECT * FROM users WHERE id=?1", vec![js(&user.id)]).await?.unwrap_or(user);
    family_payload(&db, &fresh, None).await
}

async fn family_join(req: &mut Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    if user.household_id.is_some() {
        return Ok(err(400, "Leave your current family first"));
    }
    #[derive(Deserialize)]
    struct Body { code: String }
    let body: Body = match req.json().await {
        Ok(v) => v,
        Err(_) => return Ok(err(400, "Enter an invite code")),
    };
    let code = body.code.trim().to_uppercase();
    let db = env.d1("DB")?;
    let hh: Option<Household> = one(&db, "SELECT * FROM households WHERE invite_code=?1", vec![js(&code)]).await?;
    let Some(hh) = hh else {
        return Ok(err(404, "That invite code doesn't match a family"));
    };
    let count: Option<Count> = one(&db, "SELECT COUNT(*) AS n FROM users WHERE household_id=?1", vec![js(&hh.id)]).await?;
    if count.map(|c| c.n).unwrap_or(0) >= 8 {
        return Ok(err(400, "This family is full (8 people max)"));
    }
    exec(&db, "UPDATE users SET household_id=?1 WHERE id=?2", vec![js(&hh.id), js(&user.id)]).await?;
    let fresh = one(&db, "SELECT * FROM users WHERE id=?1", vec![js(&user.id)]).await?.unwrap_or(user);
    family_payload(&db, &fresh, None).await
}

async fn family_leave(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let Some(hid) = user.household_id else {
        return Ok(err(400, "You're not in a family"));
    };
    let db = env.d1("DB")?;
    let hh: Option<Household> = one(&db, "SELECT * FROM households WHERE id=?1", vec![js(&hid)]).await?;
    let members: Vec<User> = all(&db, "SELECT * FROM users WHERE household_id=?1", vec![js(&hid)]).await?;
    exec(&db, "UPDATE users SET household_id=NULL WHERE id=?1", vec![js(&user.id)]).await?;
    if members.len() <= 1 {
        exec(&db, "DELETE FROM households WHERE id=?1", vec![js(&hid)]).await?;
    } else if let Some(hh) = hh {
        if hh.owner_id == user.id {
            if let Some(next) = members.iter().find(|m| m.id != user.id) {
                exec(&db, "UPDATE households SET owner_id=?1 WHERE id=?2", vec![js(&next.id), js(&hid)]).await?;
            }
        }
    }
    Ok(json_msg("left family", 200))
}

async fn family_rotate(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let Some(hid) = user.household_id else {
        return Ok(err(400, "You're not in a family"));
    };
    let db = env.d1("DB")?;
    let hh: Option<Household> = one(&db, "SELECT * FROM households WHERE id=?1", vec![js(&hid)]).await?;
    let Some(hh) = hh else {
        return Ok(err(404, "Family not found"));
    };
    if hh.owner_id != user.id {
        return Ok(err(403, "Only the family owner can rotate the code"));
    }
    let code = unique_code(&db).await?;
    exec(&db, "UPDATE households SET invite_code=?1 WHERE id=?2", vec![js(&code), js(&hid)]).await?;
    json(&json!({ "invite_code": code }), 200)
}

async fn member_diary(req: &Request, env: &Env, id: &str) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let db = env.d1("DB")?;
    let target: Option<User> = one(&db, "SELECT * FROM users WHERE id=?1", vec![js(id)]).await?;
    let Some(target) = target else {
        return Ok(err(404, "Member not found"));
    };
    let same_family = target.id == user.id
        || (user.household_id.is_some() && user.household_id == target.household_id);
    if !same_family {
        return Ok(err(403, "That person isn't in your family"));
    }
    if target.id != user.id && !target.share_diary {
        return Ok(err(403, "This member keeps their diary private"));
    }
    let date = query(req, "date");
    let (date, entries) = day_entries(&db, &target.id, date.as_deref()).await?;
    let mut totals = Totals::default();
    for e in &entries {
        totals.add(&Totals { kcal: e.kcal, protein: e.protein, carbs: e.carbs, fat: e.fat });
    }
    json(
        &MemberDay {
            id: target.id,
            name: target.name,
            picture: target.picture,
            date,
            calorie_target: target.calorie_target,
            totals,
            entries,
        },
        200,
    )
}

// ---------------------------------------------------------------- insights

async fn insights(req: &Request, env: &Env) -> Result<Response> {
    let Some(user) = current_user(req, env).await? else {
        return Ok(err(401, "Sign in to continue"));
    };
    let days = query(req, "days")
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(14)
        .clamp(7, 90);
    let db = env.d1("DB")?;
    let end = match query(req, "end") {
        Some(d) if is_date(&d) => d,
        _ => today(&db).await?,
    };
    let sql = format!(
        "WITH RECURSIVE dates(d) AS (
            SELECT date(?2, '-{} days')
            UNION ALL SELECT date(d, '+1 day') FROM dates WHERE d < date(?2)
         )
         SELECT dates.d AS date, COALESCE(SUM(e.kcal),0) AS kcal, COALESCE(SUM(e.protein),0) AS protein
         FROM dates LEFT JOIN diary_entries e ON e.log_date = dates.d AND e.user_id = ?1
         GROUP BY dates.d ORDER BY dates.d",
        days - 1
    );
    let days_out: Vec<DayInsight> = all(&db, &sql, vec![js(&user.id), js(&end)]).await?;
    json(&InsightsResponse { days: days_out, target: user.calorie_target }, 200)
}

// ---------------------------------------------------------------- helpers

#[derive(Deserialize, Default)]
struct Sum { kcal: f64, protein: f64, entry_count: i64 }
#[derive(Deserialize)]
struct Count { n: i64 }

fn js(s: &str) -> JsValue { JsValue::from_str(s) }
fn jn(n: f64) -> JsValue { JsValue::from_f64(n) }

fn json<T: serde::Serialize>(v: &T, status: u16) -> Result<Response> {
    let res = Response::from_json(v)?;
    Ok(if status == 200 { res } else { res.with_status(status) })
}

fn json_msg(msg: &str, status: u16) -> Response {
    let body = json!({ "message": msg });
    let mut res = Response::from_json(&body).unwrap_or_else(|_| Response::empty().unwrap());
    if status != 200 {
        res = res.with_status(status);
    }
    res
}

fn err(status: u16, message: &str) -> Response {
    let body = json!({ "error": message });
    let mut res = Response::from_json(&body).unwrap_or_else(|_| Response::empty().unwrap());
    res = res.with_status(status);
    res
}

fn redirect_to(url: &str) -> Result<Response> {
    let mut res = Response::empty()?.with_status(302);
    res.headers_mut().set("location", url)?;
    Ok(res)
}

fn set_cookie(res: &mut Response, name: &str, value: &str, max_age: i64, secure: bool) -> Result<()> {
    let mut c = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}");
    if secure { c.push_str("; Secure"); }
    res.headers_mut().append("set-cookie", &c)
}

fn cookie(req: &Request, name: &str) -> Option<String> {
    let raw = req.headers().get("cookie").ok().flatten()?;
    raw.split(';').find_map(|part| {
        let (k, v) = part.trim().split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}

fn query(req: &Request, key: &str) -> Option<String> {
    req.url().ok()?.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.to_string())
}

fn var_str(env: &Env, name: &str) -> String {
    env.var(name).map(|v| v.to_string()).unwrap_or_default()
}

fn session_secret(env: &Env) -> Result<String> {
    env.secret("SESSION_SECRET")
        .map(|s| s.to_string())
        .map_err(|_| Error::Internal("SESSION_SECRET is not set".into()))
}

fn now() -> i64 {
    (Date::now().as_millis() / 1000) as i64
}

fn new_id(prefix: &str) -> String {
    let ms = Date::now().as_millis();
    let r = (worker::js_sys::Math::random() * 1_000_000_000_000.0) as u64;
    format!("{prefix}_{ms:x}{r:x}")
}

fn invite_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    (0..6)
        .map(|_| {
            let i = (worker::js_sys::Math::random() * ALPHABET.len() as f64) as usize;
            ALPHABET[i.min(ALPHABET.len() - 1)] as char
        })
        .collect()
}

async fn unique_code(db: &D1Database) -> Result<String> {
    for _ in 0..8 {
        let code = invite_code();
        let existing: Option<Count> = one(db, "SELECT 1 AS n FROM households WHERE invite_code=?1", vec![js(&code)]).await?;
        if existing.is_none() {
            return Ok(code);
        }
    }
    Err(Error::Internal("could not allocate invite code".into()))
}

async fn current_user(req: &Request, env: &Env) -> Result<Option<User>> {
    let bearer = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.trim().to_string()));
    let Some(token) = bearer.or_else(|| cookie(req, SESSION_COOKIE)) else {
        return Ok(None);
    };
    let secret = session_secret(env)?;
    let Some(uid) = verify_session(&secret, &token, now()) else {
        return Ok(None);
    };
    let db = env.d1("DB")?;
    one(&db, "SELECT * FROM users WHERE id=?1", vec![js(&uid)]).await
}

fn origin(u: &Url) -> String {
    match u.port() {
        Some(p) => format!("{}://{}:{}", u.scheme(), u.host_str().unwrap_or("localhost"), p),
        None => format!("{}://{}", u.scheme(), u.host_str().unwrap_or("localhost")),
    }
}

fn is_meal(m: &str) -> bool {
    matches!(m, "breakfast" | "lunch" | "dinner" | "snacks")
}

fn is_date(d: &str) -> bool {
    d.len() == 10
        && d.as_bytes()[4] == b'-'
        && d.as_bytes()[7] == b'-'
        && d.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

async fn today(db: &D1Database) -> Result<String> {
    #[derive(Deserialize)]
    struct D { d: String }
    let row: Option<D> = one(db, "SELECT date('now') AS d", vec![]).await?;
    Ok(row.map(|r| r.d).unwrap_or_else(|| "2026-01-01".into()))
}

async fn all<T: DeserializeOwned>(db: &D1Database, sql: &str, binds: Vec<JsValue>) -> Result<Vec<T>> {
    let stmt = db.prepare(sql);
    let stmt = if binds.is_empty() { stmt } else { stmt.bind(&binds)? };
    stmt.all().await?.results::<T>()
}

async fn one<T: DeserializeOwned>(db: &D1Database, sql: &str, binds: Vec<JsValue>) -> Result<Option<T>> {
    let stmt = db.prepare(sql);
    let stmt = if binds.is_empty() { stmt } else { stmt.bind(&binds)? };
    stmt.first::<T>(None).await
}

async fn exec(db: &D1Database, sql: &str, binds: Vec<JsValue>) -> Result<()> {
    let stmt = db.prepare(sql);
    let stmt = if binds.is_empty() { stmt } else { stmt.bind(&binds)? };
    stmt.run().await?;
    Ok(())
}

async fn post_text(url: &str, body: &str, content_type: &str, token: Option<&str>) -> Result<String> {
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_body(Some(JsValue::from_str(body)));
    let mut req = Request::new_with_init(url, &init)?;
    req.headers_mut()?.set("content-type", content_type)?;
    if let Some(t) = token {
        req.headers_mut()?.set("authorization", &format!("Bearer {t}"))?;
    }
    let mut res = Fetch::Request(req).send().await?;
    res.text().await
}

async fn get_text(url: &str, token: Option<&str>) -> Result<String> {
    let mut req = Request::new(url, Method::Get)?;
    if let Some(t) = token {
        req.headers_mut()?.set("authorization", &format!("Bearer {t}"))?;
    }
    let mut res = Fetch::Request(req).send().await?;
    res.text().await
}
