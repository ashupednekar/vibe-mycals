use base64::Engine;
use dioxus::prelude::*;
use dioxus_router::*;
use gloo_net::http::Request;
use mycals_common::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

fn main() {
    dioxus::launch(App);
}

// ------------------------------------------------------------------ api

async fn api_get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let res = Request::get(path)
        .credentials(web_sys::RequestCredentials::SameOrigin)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json(res).await
}

async fn api_send<B: Serialize, T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let req = match method {
        "PUT" => Request::put(path),
        "PATCH" => Request::patch(path),
        _ => Request::post(path),
    };
    let res = req
        .credentials(web_sys::RequestCredentials::SameOrigin)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json(res).await
}

async fn read_json<T: DeserializeOwned>(res: gloo_net::http::Response) -> Result<T, String> {
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if status >= 400 {
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("Request failed ({status})"));
        return Err(msg);
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ state

#[derive(Clone, Copy)]
struct AppState {
    user: Signal<Option<User>>,
    config: Signal<ConfigResponse>,
    refresh: Signal<u32>,
}

impl AppState {
    fn set_user(&self, user: Option<User>) {
        let mut s = self.user;
        s.set(user);
    }
    fn bump(&self) {
        let mut r = self.refresh;
        r.set(r() + 1);
    }
}

// ------------------------------------------------------------------ routes

#[derive(Routable, Clone, PartialEq, Debug)]
#[allow(unreachable_code)]
#[rustfmt::skip]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Today {},
    #[route("/family")]
    Family {},
    #[route("/profile")]
    Profile {},
    #[end_layout]
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
fn App() -> Element {
    let mut user = use_signal(|| None::<User>);
    let mut config = use_signal(|| ConfigResponse { demo: false, google: false });
    let refresh = use_signal(|| 0_u32);
    let mut loaded = use_signal(|| false);
    use_context_provider(|| AppState { user, config, refresh });

    use_future(move || async move {
        if let Ok(c) = api_get::<ConfigResponse>("/api/config").await {
            config.set(c);
        }
        if let Ok(u) = api_get::<User>("/api/me").await {
            user.set(Some(u));
        }
        loaded.set(true);
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        if !loaded() {
            Splash {}
        } else if user().is_none() {
            Login {}
        } else if !user().unwrap().onboarded {
            Onboarding {}
        } else {
            Router::<Route> {}
        }
    }
}

#[component]
fn Splash() -> Element {
    rsx! {
        div { class: "splash",
            div { class: "splash-mark", "m" }
            p { "mycals" }
        }
    }
}

#[component]
fn Logo(compact: Option<bool>) -> Element {
    let compact = compact.unwrap_or(false);
    rsx! {
        div { class: "logo",
            div { class: "logo-mark", "m" }
            if !compact { div { class: "logo-word", "my" span { "cals" } } }
        }
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[component]
fn Avatar(name: String, size: Option<u32>) -> Element {
    let size = size.unwrap_or(36);
    rsx! {
        div {
            class: "avatar",
            style: "width:{size}px;height:{size}px;font-size:{size / 2}px",
            "{initials(&name)}"
        }
    }
}

// ------------------------------------------------------------------ login

#[component]
fn Login() -> Element {
    let state = use_context::<AppState>();
    let mut notice = use_signal(String::new);
    let config = state.config;

    let google = move |_| {
        if config().google {
            if let Some(w) = web_sys::window() {
                let _ = w.location().set_href("/api/auth/google/start");
            }
        } else {
            notice.set("Google sign-in needs GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET set as Worker secrets.".into());
        }
    };
    rsx! {
        div { class: "login-page",
            div { class: "login-art",
                div { class: "orb orb-a" }
                div { class: "orb orb-b" }
                div { class: "login-art-card",
                    div { class: "mini-row",
                        div { class: "mini-avatar", "M" }
                        div { "Maya logged lunch" }
                        div { class: "mini-kcal", "520 kcal" }
                    }
                    div { class: "mini-row",
                        div { class: "mini-avatar alt", "S" }
                        div { "Sam hit his protein goal" }
                        div { class: "mini-pill", "nice" }
                    }
                    div { class: "scan-chip", "📷 scan a plate → instant calories" }
                }
                div { class: "login-photo",
                    div { class: "login-photo-emoji", "🥗" }
                }
            }
            div { class: "login-panel",
                Logo {}
                h1 { "Calories, " span { "without the noise." } }
                p { class: "lede",
                    "Track your calories in seconds, see how your family is doing, and keep each other honest — all in one clean little app."
                }
                div { class: "auth-actions",
                    button { class: "btn btn-google", onclick: google,
                        span { class: "g-badge", "G" }
                        "Continue with Google"
                    }
                }
                if !notice().is_empty() {
                    p { class: "notice", "{notice}" }
                }
                div { class: "feature-chips",
                    span { "🍽️ 60+ foods" }
                    span { "👨‍👩‍👧 family diary" }
                    span { "✨ Gemini scanning" }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ onboarding

#[component]
fn Onboarding() -> Element {
    let state = use_context::<AppState>();
    let mut name = use_signal(|| (state.user)().map(|u| u.name).unwrap_or_default());
    let mut sex = use_signal(|| "female".to_string());
    let mut birth_year = use_signal(|| "1995".to_string());
    let mut height = use_signal(|| "170".to_string());
    let mut weight = use_signal(|| "70".to_string());
    let mut activity = use_signal(|| "moderate".to_string());
    let mut goal = use_signal(|| "lose".to_string());
    let mut error = use_signal(String::new);

    let save = move |_| {
        let input = ProfileInput {
            name: Some(name()),
            sex: Some(sex()),
            birth_year: birth_year().parse().ok(),
            height_cm: height().parse().ok(),
            weight_kg: weight().parse().ok(),
            activity: Some(activity()),
            goal: Some(goal()),
            calorie_target: None,
            share_diary: Some(true),
            units: None,
            onboarded: Some(true),
        };
        spawn(async move {
            match api_send::<ProfileInput, User>("PUT", "/api/profile", &input).await {
                Ok(u) => state.set_user(Some(u)),
                Err(e) => error.set(e),
            }
        });
    };

    rsx! {
        div { class: "onboard",
            div { class: "onboard-card",
                Logo {}
                div { class: "step-label", "Let's set your baseline" }
                h1 { "A few numbers, then you're in." }
                p { class: "muted", "We use Mifflin-St Jeor to set a starting calorie target. You can change it later." }
                div { class: "form-grid",
                    label { "Your name"
                        input { value: "{name}", oninput: move |e| name.set(e.value()) }
                    }
                    label { "Sex"
                        select { value: "{sex}", onchange: move |e| sex.set(e.value()),
                            option { value: "female", "Female" }
                            option { value: "male", "Male" }
                        }
                    }
                    label { "Birth year"
                        input { r#type: "number", value: "{birth_year}", oninput: move |e| birth_year.set(e.value()) }
                    }
                    label { "Height (cm)"
                        input { r#type: "number", value: "{height}", oninput: move |e| height.set(e.value()) }
                    }
                    label { "Weight (kg)"
                        input { r#type: "number", value: "{weight}", oninput: move |e| weight.set(e.value()) }
                    }
                    label { "Activity"
                        select { value: "{activity}", onchange: move |e| activity.set(e.value()),
                            option { value: "sedentary", "Sedentary" }
                            option { value: "light", "Lightly active" }
                            option { value: "moderate", "Moderately active" }
                            option { value: "active", "Very active" }
                        }
                    }
                    label { class: "full", "Goal"
                        div { class: "segmented",
                            for (value, label) in [("lose", "Lose"), ("maintain", "Maintain"), ("gain", "Gain")] {
                                button {
                                    r#type: "button",
                                    class: if goal() == value { "seg active" } else { "seg" },
                                    onclick: move |_| goal.set(value.to_string()),
                                    "{label}"
                                }
                            }
                        }
                    }
                }
                if !error().is_empty() { p { class: "notice", "{error}" } }
                button { class: "btn btn-primary wide", onclick: save, "Start tracking" }
            }
        }
    }
}

// ------------------------------------------------------------------ shell

#[component]
fn Shell() -> Element {
    let state = use_context::<AppState>();
    let user = (state.user)().unwrap();
    let current = use_route::<Route>();
    let mut busy = use_signal(|| false);

    let logout = move |_| {
        busy.set(true);
        spawn(async move {
            let _ = api_send::<serde_json::Value, serde_json::Value>(
                "POST",
                "/api/logout",
                &serde_json::json!({}),
            )
            .await;
            state.set_user(None);
        });
    };

    let nav = |route: Route, label: &'static str, icon: &'static str| {
        let active = matches!(
            (&route, &current),
            (Route::Today {}, Route::Today {})
                | (Route::Family {}, Route::Family {})
                | (Route::Profile {}, Route::Profile {})
        );
        rsx! {
            Link {
                to: route,
                class: if active { "nav-link active" } else { "nav-link" },
                span { class: "nav-icon", "{icon}" }
                span { "{label}" }
            }
        }
    };

    rsx! {
        div { class: "shell",
            aside { class: "sidebar",
                Logo {}
                nav { class: "nav",
                    {nav(Route::Today {}, "Today", "◉")}
                    {nav(Route::Family {}, "Family", "◍")}
                    {nav(Route::Profile {}, "You", "◐")}
                }
                div { class: "sidebar-foot",
                    div { class: "who",
                        Avatar { name: user.name.clone(), size: 38 }
                        div {
                            div { class: "who-name", "{user.name}" }
                            div { class: "who-sub", "family diary" }
                        }
                    }
                    button { class: "ghost-btn", onclick: logout, disabled: busy(), "Sign out" }
                }
            }
            main { class: "main",
                Outlet::<Route> {}
            }
            nav { class: "tabbar",
                {nav(Route::Today {}, "Today", "◉")}
                {nav(Route::Family {}, "Family", "◍")}
                {nav(Route::Profile {}, "You", "◐")}
            }
        }
    }
}

// ------------------------------------------------------------------ today

fn local_today() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

fn shift_date(date: &str, days: i32) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(&format!("{date}T12:00:00")));
    d.set_time(d.get_time() + days as f64 * 86_400_000.0);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

fn pretty_date(date: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(&format!("{date}T12:00:00")));
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let today = local_today();
    if date == today {
        return "Today".into();
    }
    if date == shift_date(&today, -1) {
        return "Yesterday".into();
    }
    format!(
        "{}, {} {}",
        days[d.get_day() as usize % 7],
        months[d.get_month() as usize % 12],
        d.get_date()
    )
}

#[component]
fn Today() -> Element {
    let state = use_context::<AppState>();
    let mut date = use_signal(local_today);
    let refresh = state.refresh;
    let mut meal_modal = use_signal(|| None::<String>);
    let mut scan_open = use_signal(|| false);
    let mut error = use_signal(String::new);

    let diary = use_resource(move || async move {
        let _ = refresh();
        api_get::<DiaryResponse>(&format!("/api/diary?date={}", date())).await
    });
    let insights = use_resource(move || async move {
        let _ = refresh();
        api_get::<InsightsResponse>(&format!("/api/insights?days=7&end={}", local_today())).await
    });

    let data: Option<DiaryResponse> = match &*diary.read() {
        Some(Ok(v)) => Some(v.clone()),
        _ => None,
    };
    let load_error: Option<String> = match &*diary.read() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };
    let week: Vec<DayInsight> = match &*insights.read() {
        Some(Ok(v)) => v.days.clone(),
        _ => vec![],
    };

    let bump = move || { let mut r = refresh; r.set(r() + 1); };
    if let Some(d) = data {
        let target = d.targets.calorie.max(1) as f64;
        let pct = (d.totals.kcal / target).clamp(0.0, 1.25);
        let remaining = (d.targets.calorie as f64 - d.totals.kcal).round();
        let meals = [
            ("breakfast", "Breakfast", "☀️"),
            ("lunch", "Lunch", "🥗"),
            ("dinner", "Dinner", "🌙"),
            ("snacks", "Snacks", "🍎"),
        ];
        let max_day = week
            .iter()
            .map(|d| d.kcal)
            .fold(d.targets.calorie as f64, f64::max)
            .max(1.0);

        rsx! {
            div { class: "page",
                header { class: "page-head",
                    div {
                        div { class: "eyebrow", "Your diary" }
                        h1 { class: "date-title", "{pretty_date(&d.date)}" }
                        div { class: "date-nav",
                            button { class: "chip", onclick: move |_| date.set(shift_date(&date(), -1)), "‹" }
                            button { class: "chip", onclick: move |_| date.set(local_today()), "Today" }
                            button {
                                class: "chip",
                                disabled: d.date >= local_today(),
                                onclick: move |_| date.set(shift_date(&date(), 1)),
                                "›"
                            }
                        }
                    }
                    div { class: "head-actions",
                        button { class: "btn btn-scan", onclick: move |_| scan_open.set(true),
                            span { "📷" } "Scan food"
                        }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |_| meal_modal.set(Some("lunch".into())),
                            "+ Add food"
                        }
                    }
                }
                if !error().is_empty() { div { class: "banner", "{error}" } }
                div { class: "today-grid",
                    section { class: "card calorie-card",
                        div { class: "card-head",
                            span { class: "card-title", "Calories" }
                            span { class: "card-sub", "{d.targets.calorie} goal" }
                        }
                        div { class: "ring-wrap",
                            svg { view_box: "0 0 120 120", class: "ring",
                                circle { cx: "60", cy: "60", r: "50", fill: "none", stroke: "#ecebe4", stroke_width: "10" }
                                circle {
                                    cx: "60", cy: "60", r: "50", fill: "none",
                                    stroke: if remaining < 0.0 { "#e2704c" } else { "#1f7a4c" },
                                    stroke_width: "10", stroke_linecap: "round",
                                    stroke_dasharray: format!("{:.1} 320", (pct.min(1.0) * 314.0)),
                                    transform: "rotate(-90 60 60)"
                                }
                            }
                            div { class: "ring-center",
                                div { class: "ring-num", "{d.totals.kcal.round()}" }
                                div { class: "ring-label", "of {d.targets.calorie}" }
                                div { class: if remaining < 0.0 { "ring-left over" } else { "ring-left" },
                                if remaining >= 0.0 { "{remaining} left" } else { {format!("{} over", remaining.abs())} }
                            }
                            }
                        }
                    }
                    section { class: "card side-card",
                        div { class: "card-head",
                            span { class: "card-title", "This week" }
                            span { class: "card-sub", "kcal / day" }
                        }
                        div { class: "bars",
                            for day in week.iter() {
                                div { class: "bar-col", title: "{day.date}: {day.kcal.round()} kcal",
                                    div { class: "bar-track",
                                        div {
                                            class: "bar",
                                            style: format!("height:{:.0}%", (day.kcal / max_day * 100.0).clamp(3.0, 100.0))
                                        }
                                    }
                                    div { class: "bar-day", {day.date[8..].to_string()} }
                                }
                            }
                        }
                        div { class: "week-foot",
                            span { {format!("Avg {:.0} kcal", week.iter().map(|d| d.kcal).sum::<f64>() / week.len().max(1) as f64)} }
                            span { "Goal {d.targets.calorie}" }
                        }
                    }
                }

                div { class: "meals",
                    for (key, label, emoji) in meals {
                        section { class: "card meal-card",
                            div { class: "meal-head",
                                div { class: "meal-title",
                                    span { class: "meal-emoji", "{emoji}" }
                                    span { "{label}" }
                                }
                                div { class: "meal-right",
                                    span { class: "meal-kcal",
                                        {format!("{:.0} kcal", { let t = d.entries.iter().filter(|e| e.meal == key).map(|e| e.kcal).sum::<f64>(); if t > 0.0 { t } else { 0.0 } })}
                                    }
                                    button { class: "add-mini", onclick: move |_| meal_modal.set(Some(key.to_string())), "+" }
                                }
                            }
                            if d.entries.iter().filter(|e| e.meal == key).count() == 0 {
                                div { class: "meal-empty", onclick: move |_| meal_modal.set(Some(key.to_string())),
                                    "Nothing logged yet — tap to add"
                                }
                            }
                            for entry in d.entries.iter().filter(|e| e.meal == key) {
                                div { class: "entry",
                                    div { class: "entry-main",
                                        div { class: "entry-name", "{entry.name}" }
                                        div { class: "entry-meta", "{entry.grams:.0} g" }
                                    }
                                    div { class: "entry-kcal", "{entry.kcal:.0}" }
                                    button {
                                        class: "entry-del",
                                        title: "Remove",
                                        onclick: {
                                            let id = entry.id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    if let Err(e) = api_send::<serde_json::Value, serde_json::Value>(
                                                        "DELETE", &format!("/api/diary/entries/{id}"), &serde_json::json!({})
                                                    ).await { error.set(e); } else { bump(); }
                                                });
                                            }
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(meal) = meal_modal() {
                    AddFoodModal {
                        meal,
                        date: d.date.clone(),
                        on_close: move |_| meal_modal.set(None),
                        on_added: move |_| { meal_modal.set(None); bump(); },
                    }
                }
                if scan_open() {
                    ScanModal {
                        date: d.date.clone(),
                        on_close: move |_| scan_open.set(false),
                        on_added: move |_| { scan_open.set(false); bump(); },
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "page",
                header { class: "page-head",
                    div { h1 { class: "date-title", "{pretty_date(&date())}" } }
                }
                if let Some(e) = load_error {
                    div { class: "banner", "{e}" }
                } else {
                    div { class: "card loading-card", "Loading your day…" }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ add food

#[component]
fn AddFoodModal(
    meal: String,
    date: String,
    on_close: EventHandler<()>,
    on_added: EventHandler<()>,
) -> Element {
    let mut q = use_signal(String::new);
    let mut selected = use_signal(|| None::<Food>);
    let mut grams = use_signal(|| "100".to_string());
    let mut barcode = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut custom = use_signal(|| false);
    let mut cname = use_signal(String::new);
    let mut ckcal = use_signal(|| "300".to_string());
    let mut cp = use_signal(|| "20".to_string());
    let mut cc = use_signal(|| "30".to_string());
    let mut cf = use_signal(|| "10".to_string());

    let foods = use_resource(move || async move {
        let query = String::from(js_sys::encode_uri_component(&q()));
        api_get::<Vec<Food>>(&format!("/api/foods?q={query}")).await
    });
    let list: Vec<Food> = match &*foods.read() {
        Some(Ok(v)) => v.clone(),
        _ => vec![],
    };
    let lookup = move || {
        let code = barcode().trim().to_string();
        if code.is_empty() { return; }
        spawn(async move {
            match api_get::<Food>(&format!("/api/foods/barcode/{code}")).await {
                Ok(f) => { selected.set(Some(f)); error.set(String::new()); }
                Err(e) => error.set(e),
            }
        });
    };
    let submit_date = date.clone();
    let submit_meal = meal.clone();
    let submit = move |_| {
        busy.set(true);
        let date = submit_date.clone();
        let meal = submit_meal.clone();
        spawn(async move {
            let result = if let Some(food) = selected() {
                let body = EntryInput {
                    date, meal, food_id: Some(food.id), name: food.name,
                    grams: grams().parse().unwrap_or(100.0),
                    kcal: 0.0, protein: 0.0, carbs: 0.0, fat: 0.0,
                };
                api_send::<EntryInput, DiaryEntry>("POST", "/api/diary/entries", &body).await
            } else {
                let body = EntryInput {
                    date, meal, food_id: None, name: cname(),
                    grams: 100.0,
                    kcal: ckcal().parse().unwrap_or(0.0),
                    protein: cp().parse().unwrap_or(0.0),
                    carbs: cc().parse().unwrap_or(0.0),
                    fat: cf().parse().unwrap_or(0.0),
                };
                api_send::<EntryInput, DiaryEntry>("POST", "/api/diary/entries", &body).await
            };
            match result {
                Ok(_) => on_added.call(()),
                Err(e) => { error.set(e); busy.set(false); }
            }
        });
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                div { class: "modal-head",
                    div {
                        div { class: "eyebrow", "Add to {meal}" }
                        h2 { "What did you eat?" }
                    }
                    button { class: "x", onclick: move |_| on_close.call(()), "×" }
                }
                div { class: "modal-tabs",
                    button { class: if !custom() { "tab active" } else { "tab" }, onclick: move |_| custom.set(false), "Search foods" }
                    button { class: if custom() { "tab active" } else { "tab" }, onclick: move |_| custom.set(true), "Quick entry" }
                }
                if !custom() {
                    div { class: "food-search",
                        input { placeholder: "Search chicken, oats, dal…", value: "{q}", oninput: move |e| q.set(e.value()) }
                        input {
                            placeholder: "Barcode",
                            class: "barcode-input",
                            value: "{barcode}",
                            oninput: move |e| barcode.set(e.value()),
                            onkeydown: move |e| if e.key() == Key::Enter { lookup() },
                        }
                        button { class: "btn btn-ghost", onclick: move |_| lookup(), "Look up" }
                    }
                    if let Some(f) = selected() {
                        div { class: "selected-food",
                            div {
                                div { class: "sel-name", "{f.name}" }
                                div { class: "muted tiny", "{f.kcal100:.0} kcal / 100 g · P {f.protein100:.0} C {f.carbs100:.0} F {f.fat100:.0}" }
                            }
                            div { class: "grams-row",
                                label { "Portion (g)"
                                    input { r#type: "number", value: "{grams}", oninput: move |e| grams.set(e.value()) }
                                }
                                div { class: "sel-kcal",
                                    {format!("{:.0} kcal", f.kcal100 * grams().parse::<f64>().unwrap_or(0.0) / 100.0)}
                                }
                            }
                            button { class: "btn btn-primary", disabled: busy(), onclick: submit, "Add to {meal}" }
                        }
                    } else {
                        div { class: "food-list",
                            if list.is_empty() {
                                div { class: "meal-empty", "No matches. Try Quick entry." }
                            }
                            for f in list.iter() {
                                button {
                                    class: "food-option",
                                    onclick: {
                                        let f = f.clone();
                                        move |_| {
                                            selected.set(Some(f.clone()));
                                            grams.set(f.serving_grams.unwrap_or(100.0).to_string());
                                        }
                                    },
                                    div { class: "food-name", "{f.name}" }
                                    div { class: "food-meta",
                                        if f.is_public { "public · " } else { "custom · " }
                                        "{f.kcal100:.0} kcal/100 g"
                                        if f.shared { span { class: "pill", "family" } }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "form-grid",
                        label { class: "full", "Name"
                            input { value: "{cname}", placeholder: "e.g. Mom's lasagna", oninput: move |e| cname.set(e.value()) }
                        }
                        label { "Calories"
                            input { r#type: "number", value: "{ckcal}", oninput: move |e| ckcal.set(e.value()) }
                        }
                        label { "Protein (g)"
                            input { r#type: "number", value: "{cp}", oninput: move |e| cp.set(e.value()) }
                        }
                        label { "Carbs (g)"
                            input { r#type: "number", value: "{cc}", oninput: move |e| cc.set(e.value()) }
                        }
                        label { "Fat (g)"
                            input { r#type: "number", value: "{cf}", oninput: move |e| cf.set(e.value()) }
                        }
                    }
                    button { class: "btn btn-primary wide", disabled: busy(), onclick: submit, "Add to {meal}" }
                }
                if !error().is_empty() { p { class: "notice", "{error}" } }
            }
        }
    }
}

// ------------------------------------------------------------------ scan

#[component]
fn ScanModal(date: String, on_close: EventHandler<()>, on_added: EventHandler<()>) -> Element {
    let mut preview = use_signal(|| None::<String>);
    let mut image = use_signal(String::new);
    let mut mime = use_signal(|| "image/jpeg".to_string());
    let mut result = use_signal(|| None::<ScanResult>);
    let mut error = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut meal = use_signal(|| "lunch".to_string());
    let mut saving = use_signal(|| false);

    let pick = move |evt: Event<FormData>| {
        let files = evt.files();
        let Some(file) = files.first().cloned() else { return };
        let m = file.content_type().unwrap_or_else(|| "image/jpeg".into());
        mime.set(m.clone());
        spawn(async move {
            if let Ok(bytes) = file.read_bytes().await {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                preview.set(Some(format!("data:{m};base64,{b64}")));
                image.set(b64);
                result.set(None);
                error.set(String::new());
            }
        });
    };
    let analyze = move |_| {
        if image().is_empty() { error.set("Pick a photo first".into()); return; }
        busy.set(true);
        error.set(String::new());
        spawn(async move {
            let body = ScanInput { image_base64: image(), mime_type: mime() };
            match api_send::<ScanInput, ScanResult>("POST", "/api/scan", &body).await {
                Ok(r) => result.set(Some(r)),
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };
    let add_all = move |_| {
        let Some(r) = result() else { return };
        saving.set(true);
        let date = date.clone();
        let m = meal();
        spawn(async move {
            for item in r.items.iter() {
                let body = EntryInput {
                    date: date.clone(),
                    meal: m.clone(),
                    food_id: None,
                    name: if r.items.len() > 1 { format!("{} · {}", r.name, item.name) } else { item.name.clone() },
                    grams: item.grams,
                    kcal: item.kcal,
                    protein: item.protein,
                    carbs: item.carbs,
                    fat: item.fat,
                };
                if let Err(e) = api_send::<EntryInput, DiaryEntry>("POST", "/api/diary/entries", &body).await {
                    error.set(e);
                    saving.set(false);
                    return;
                }
            }
            on_added.call(());
        });
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div { class: "modal modal-scan", onclick: move |e| e.stop_propagation(),
                div { class: "modal-head",
                    div {
                        div { class: "eyebrow", "Gemini vision" }
                        h2 { "Scan a plate or label" }
                    }
                    button { class: "x", onclick: move |_| on_close.call(()), "×" }
                }
                if let Some(src) = preview() {
                    div { class: "scan-preview",
                        img { src, alt: "Food to analyze" }
                        label { class: "re-pick",
                            "Change photo"
                            input { r#type: "file", accept: "image/*", capture: "environment", onchange: pick, class: "hidden-file" }
                        }
                    }
                } else {
                    label { class: "scan-drop",
                        div { class: "scan-big", "📷" }
                        div { "Take a photo or choose from your library" }
                        div { class: "muted tiny", "Labels, plates and packaged food all work" }
                        input { r#type: "file", accept: "image/*", capture: "environment", onchange: pick, class: "hidden-file" }
                    }
                }
                if let Some(r) = result() {
                    div { class: "scan-result",
                        div { class: "scan-result-head",
                            div {
                                div { class: "scan-name", "{r.name}" }
                                div { class: "muted tiny", "{r.description}" }
                            }
                            div { class: "scan-total", {format!("{:.0} kcal", r.items.iter().map(|i| i.kcal).sum::<f64>())} }
                        }
                        for item in r.items.iter() {
                            div { class: "scan-item",
                                div { "{item.name} · {item.grams:.0} g" }
                                div { class: "scan-item-kcal", "{item.kcal:.0}" }
                            }
                        }
                        div { class: "scan-actions",
                            select { value: "{meal}", onchange: move |e| meal.set(e.value()),
                                option { value: "breakfast", "Breakfast" }
                                option { value: "lunch", "Lunch" }
                                option { value: "dinner", "Dinner" }
                                option { value: "snacks", "Snacks" }
                            }
                            button { class: "btn btn-primary", disabled: saving(), onclick: add_all,
                                if saving() { "Adding…" } else { "Add {r.items.len()} to diary" }
                            }
                            button { class: "ghost-btn", onclick: analyze, "Scan again" }
                        }
                    }
                } else {
                    div { class: "scan-cta",
                        button { class: "btn btn-primary", disabled: busy() || image().is_empty(), onclick: analyze,
                            if busy() { "Reading your plate…" } else { "Detect nutrition" }
                        }
                        p { class: "muted tiny", "Estimates from Gemini. Check the numbers before logging." }
                    }
                }
                if !error().is_empty() { p { class: "notice", "{error}" } }
            }
        }
    }
}

// ------------------------------------------------------------------ family

#[component]
fn Family() -> Element {
    let state = use_context::<AppState>();
    let user = (state.user)().unwrap();
    let refresh = state.refresh;
    let mut error = use_signal(String::new);
    let mut join_code = use_signal(String::new);
    let mut create_name = use_signal(String::new);
    let mut member = use_signal(|| None::<FamilyMember>);
    let mut busy = use_signal(|| false);
    let date = local_today();

    let fetch_date = date.clone();
    let family = use_resource(move || {
        let fetch_date = fetch_date.clone();
        async move {
            let _ = refresh();
            api_get::<FamilyResponse>(&format!("/api/family?date={fetch_date}")).await
        }
    });
    let fam: Option<FamilyResponse> = match &*family.read() {
        Some(Ok(v)) => Some(v.clone()),
        _ => None,
    };
    let load_error: Option<String> = match &*family.read() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };

    let create = move |_| {
        busy.set(true);
        let name = create_name();
        spawn(async move {
            let body = serde_json::json!({ "name": name });
            match api_send::<_, FamilyResponse>("POST", "/api/family", &body).await {
                Ok(_) => state.bump(),
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };
    let join = move |_| {
        busy.set(true);
        let code = join_code();
        spawn(async move {
            let body = serde_json::json!({ "code": code });
            match api_send::<_, FamilyResponse>("POST", "/api/family/join", &body).await {
                Ok(_) => state.bump(),
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };
    let leave = move |_| {
        spawn(async move {
            match api_send::<serde_json::Value, serde_json::Value>(
                "POST", "/api/family/leave", &serde_json::json!({})
            ).await {
                Ok(_) => state.bump(),
                Err(e) => error.set(e),
            }
        });
    };
    let rotate = move |_| {
        spawn(async move {
            match api_send::<serde_json::Value, serde_json::Value>(
                "POST", "/api/family/rotate", &serde_json::json!({})
            ).await {
                Ok(_) => state.bump(),
                Err(e) => error.set(e),
            }
        });
    };
    let copy = move |code: String| {
        let _ = dioxus::document::eval(&format!(
            "navigator.clipboard && navigator.clipboard.writeText({})",
            serde_json::to_string(&code).unwrap_or_default()
        ));
    };

    rsx! {
        div { class: "page",
            header { class: "page-head",
                div {
                    div { class: "eyebrow", "Family sharing" }
                    h1 { class: "date-title", "Your people" }
                }
                if let Some(f) = &fam {
                    if f.household.is_some() {
                        button { class: "ghost-btn", onclick: leave, "Leave family" }
                    }
                }
            }
            if !error().is_empty() { div { class: "banner", "{error}" } }
            if let Some(e) = load_error { div { class: "banner", "{e}" } }
            if let Some(f) = fam {
                if let Some(h) = f.household {
                    section { class: "card family-hero",
                        div {
                            div { class: "eyebrow", "Invite code" }
                            div { class: "invite-row",
                                div { class: "invite-code", "{h.invite_code}" }
                                button { class: "chip", onclick: { let c = h.invite_code.clone(); move |_| copy(c.clone()) }, "Copy" }
                            }
                            p { class: "muted tiny", "Send this to your people. Up to 8 humans." }
                        }
                        div { class: "family-hero-art",
                            div { class: "stack-avatar", "M" }
                            div { class: "stack-avatar s2", "S" }
                            div { class: "stack-avatar s3", "A" }
                        }
                    }
                    if f.is_owner {
                        div { class: "inline-actions",
                            button { class: "ghost-btn", onclick: rotate, "Rotate invite code" }
                        }
                    }
                    h2 { class: "section-title", "Today · {pretty_date(&date)}" }
                    div { class: "family-grid",
                        for m in f.members.iter() {
                            button {
                                class: "card member-card",
                                onclick: {
                                    let m = m.clone();
                                    move |_| member.set(Some(m.clone()))
                                },
                                div { class: "member-top",
                                    if m.id == user.id { div { class: "avatar me", "YOU" } } else { Avatar { name: m.name.clone(), size: 42 } }
                                    div { class: "member-name-wrap",
                                        div { class: "member-name", "{m.name}" if m.id == user.id { span { class: "you-tag", "you" } } }
                                        div { class: "muted tiny", {format!("{} kcal logged", m.kcal.round())} }
                                    }
                                    if !m.share_diary && m.id != user.id {
                                        span { class: "lock", "🔒" }
                                    } else {
                                        span { class: "member-pct", {format!("{:.0}%", (m.kcal / m.calorie_target.max(1) as f64 * 100.0).min(999.0))} }
                                    }
                                }
                                div { class: "member-track",
                                    div {
                                        class: if m.kcal > m.calorie_target as f64 { "member-fill over" } else { "member-fill" },
                                        style: format!("width:{:.0}%", (m.kcal / m.calorie_target.max(1) as f64 * 100.0).clamp(0.0, 100.0))
                                    }
                                }
                                div { class: "member-foot",
                                    span { "{m.kcal.round()} / {m.calorie_target} kcal" }
                                }
                            }
                        }
                    }
                    p { class: "muted tiny family-note",
                        "Tap a family member to see their meals. Members can switch sharing off in their profile."
                    }
                } else {
                    div { class: "join-grid",
                        section { class: "card join-card",
                            div { class: "join-emoji", "🌱" }
                            h2 { "Start a family" }
                            p { class: "muted", "Create a shared space, then invite up to 7 people with a code." }
                            input { placeholder: "Family name (optional)", value: "{create_name}", oninput: move |e| create_name.set(e.value()) }
                            button { class: "btn btn-primary wide", disabled: busy(), onclick: create, "Create family" }
                        }
                        section { class: "card join-card",
                            div { class: "join-emoji", "🔑" }
                            h2 { "Join a family" }
                            p { class: "muted", "Got a code from someone? Enter it to share diaries." }
                            input { placeholder: "e.g. DEMO42", value: "{join_code}", oninput: move |e| join_code.set(e.value().to_uppercase()) }
                            button { class: "btn btn-ghost wide", disabled: busy(), onclick: join, "Join with code" }
                        }
                    }
                }
            } else {
                div { class: "card loading-card", "Loading your family…" }
            }
            if let Some(m) = member() {
                MemberModal {
                    member: m,
                    date: date.clone(),
                    on_close: move |_| member.set(None),
                }
            }
        }
    }
}

#[component]
fn MemberModal(member: FamilyMember, date: String, on_close: EventHandler<()>) -> Element {
    let id = member.id.clone();
    let fetch_date = date.clone();
    let day = use_resource(move || {
        let id = id.clone();
        let fetch_date = fetch_date.clone();
        async move { api_get::<MemberDay>(&format!("/api/family/members/{id}/diary?date={fetch_date}")).await }
    });
    let data: Option<MemberDay> = match &*day.read() {
        Some(Ok(v)) => Some(v.clone()),
        _ => None,
    };
    let err: Option<String> = match &*day.read() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };
    rsx! {
        div { class: "modal-backdrop", onclick: move |_| on_close.call(()),
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                div { class: "modal-head",
                    div {
                        div { class: "eyebrow", "Family diary" }
                        h2 { "{member.name} · {pretty_date(&date)}" }
                    }
                    button { class: "x", onclick: move |_| on_close.call(()), "×" }
                }
                if let Some(d) = data {
                    div { class: "member-summary",
                        div { class: "ring-num", "{d.totals.kcal.round()}" }
                        div { class: "muted tiny", "of {d.calorie_target} kcal" }
                    }
                    div { class: "member-entries",
                        if d.entries.is_empty() {
                            div { class: "meal-empty", "Nothing logged on this day." }
                        }
                        for e in d.entries.iter() {
                            div { class: "entry",
                                div { class: "entry-main",
                                    div { class: "entry-name", "{e.name}" }
                                    div { class: "entry-meta", "{e.meal} · {e.grams:.0} g" }
                                }
                                div { class: "entry-kcal", "{e.kcal:.0}" }
                            }
                        }
                    }
                } else if let Some(e) = err {
                    div { class: "banner", "{e}" }
                } else {
                    div { class: "loading-card", "Loading…" }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ profile

#[component]
fn Profile() -> Element {
    let state = use_context::<AppState>();
    let user = (state.user)().unwrap();
    let mut name = use_signal(|| user.name.clone());
    let mut sex = use_signal(|| user.sex.clone());
    let mut birth_year = use_signal(|| user.birth_year.unwrap_or(1995).to_string());
    let mut height = use_signal(|| user.height_cm.unwrap_or(170.0).to_string());
    let mut weight = use_signal(|| user.weight_kg.unwrap_or(70.0).to_string());
    let mut activity = use_signal(|| user.activity.clone());
    let mut goal = use_signal(|| user.goal.clone());
    let mut calorie_target = use_signal(|| user.calorie_target.to_string());
    let mut share = use_signal(|| user.share_diary);
    let mut saved = use_signal(String::new);
    let mut error = use_signal(String::new);

    let save = move |_| {
        let input = ProfileInput {
            name: Some(name()),
            sex: Some(sex()),
            birth_year: birth_year().parse().ok(),
            height_cm: height().parse().ok(),
            weight_kg: weight().parse().ok(),
            activity: Some(activity()),
            goal: Some(goal()),
            calorie_target: calorie_target().parse().ok(),
            share_diary: Some(share()),
            units: None,
            onboarded: Some(true),
        };
        spawn(async move {
            match api_send::<ProfileInput, User>("PUT", "/api/profile", &input).await {
                Ok(u) => { state.set_user(Some(u)); saved.set("Saved".into()); error.set(String::new()); }
                Err(e) => error.set(e),
            }
        });
    };
    let toggle_share = move |_| {
        let next = !share();
        share.set(next);
        let inputs = ProfileInput {
            name: None, sex: None, birth_year: None, height_cm: None, weight_kg: None,
            activity: None, goal: None, calorie_target: None, share_diary: Some(next), units: None, onboarded: None,
        };
        spawn(async move {
            match api_send::<ProfileInput, User>("PUT", "/api/profile", &inputs).await {
                Ok(u) => state.set_user(Some(u)),
                Err(e) => error.set(e),
            }
        });
    };
    let logout = move |_| {
        spawn(async move {
            let _ = api_send::<serde_json::Value, serde_json::Value>(
                "POST", "/api/logout", &serde_json::json!({})
            ).await;
            state.set_user(None);
        });
    };

    rsx! {
        div { class: "page",
            header { class: "page-head",
                div {
                    div { class: "eyebrow", "Your settings" }
                    h1 { class: "date-title", "Hi, {user.name}" }
                }
                button { class: "ghost-btn", onclick: logout, "Sign out" }
            }
            div { class: "profile-grid",
                section { class: "card profile-card",
                    div { class: "card-head", span { class: "card-title", "Profile" }, span { class: "card-sub", "drives your targets" } }
                    div { class: "form-grid",
                        label { "Name"
                            input { value: "{name}", oninput: move |e| name.set(e.value()) }
                        }
                        label { "Sex"
                            select { value: "{sex}", onchange: move |e| sex.set(e.value()),
                                option { value: "female", "Female" }
                                option { value: "male", "Male" }
                            }
                        }
                        label { "Birth year"
                            input { r#type: "number", value: "{birth_year}", oninput: move |e| birth_year.set(e.value()) }
                        }
                        label { "Height (cm)"
                            input { r#type: "number", value: "{height}", oninput: move |e| height.set(e.value()) }
                        }
                        label { "Weight (kg)"
                            input { r#type: "number", value: "{weight}", oninput: move |e| weight.set(e.value()) }
                        }
                        label { "Activity"
                            select { value: "{activity}", onchange: move |e| activity.set(e.value()),
                                option { value: "sedentary", "Sedentary" }
                                option { value: "light", "Light" }
                                option { value: "moderate", "Moderate" }
                                option { value: "active", "Very active" }
                            }
                        }
                        label { class: "full", "Daily calorie target"
                            input { r#type: "number", value: "{calorie_target}", oninput: move |e| calorie_target.set(e.value()) }
                        }
                        label { class: "full", "Goal"
                            div { class: "segmented",
                                for (value, label) in [("lose", "Lose"), ("maintain", "Maintain"), ("gain", "Gain")] {
                                    button {
                                        r#type: "button",
                                        class: if goal() == value { "seg active" } else { "seg" },
                                        onclick: move |_| goal.set(value.to_string()),
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "inline-actions",
                        button { class: "btn btn-primary", onclick: save, "Save changes" }
                        if !saved().is_empty() { span { class: "saved", "✓ {saved}" } }
                    }
                }
                div { class: "profile-side",
                    section { class: "card targets-card",
                        div { class: "card-head", span { class: "card-title", "Daily targets" }, span { class: "card-sub", "editable" } }
                        div { class: "target-grid one",
                            div { div { class: "target-num", "{user.calorie_target}" } div { class: "muted tiny", "calories / day" } }
                        }
                    }
                    section { class: "card share-card",
                        div { class: "share-row",
                            div {
                                div { class: "card-title", "Share my diary" }
                                div { class: "muted tiny", "Let your family see your meals and calories." }
                            }
                            button {
                                class: if share() { "toggle on" } else { "toggle" },
                                onclick: toggle_share,
                                div { class: "knob" }
                            }
                        }
                    }
                    section { class: "card note-card",
                        div { class: "note-emoji", "✨" }
                        p { "Scanning uses Gemini through a Cloudflare Worker secret. Your photo is processed once and never stored." }
                    }
                }
            }
            if !error().is_empty() { div { class: "banner", "{error}" } }
        }
    }
}

// ------------------------------------------------------------------ fallback

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div { class: "page",
            h1 { "Lost?" }
            p { class: "muted", "No page at /{segments.join(\"/\")}." }
            Link { to: Route::Today {}, class: "btn btn-primary", "Back to today" }
        }
    }
}
