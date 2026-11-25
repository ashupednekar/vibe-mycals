use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Targets {
    pub calorie: i64,
    pub protein: i64,
    pub carbs: i64,
    pub fat: i64,
}

/// Mifflin-St Jeor BMR, activity factor, goal adjustment and a protein-forward macro split.
pub fn calc_targets(
    sex: &str,
    age: i64,
    height_cm: f64,
    weight_kg: f64,
    activity: &str,
    goal: &str,
) -> Targets {
    let age = age.clamp(14, 100) as f64;
    let height = height_cm.clamp(120.0, 230.0);
    let weight = weight_kg.clamp(35.0, 300.0);
    let male = sex == "male";
    let bmr = 10.0 * weight + 6.25 * height - 5.0 * age + if male { 5.0 } else { -161.0 };
    let factor = match activity {
        "sedentary" => 1.2,
        "light" => 1.375,
        "active" => 1.725,
        "athlete" => 1.9,
        _ => 1.55,
    };
    let adjust = match goal {
        "lose" => -450.0,
        "gain" => 300.0,
        _ => 0.0,
    };
    let calorie = (bmr * factor + adjust).clamp(1200.0, 4500.0).round();
    let protein = match goal {
        "lose" => weight * 2.0,
        "gain" => weight * 1.8,
        _ => weight * 1.6,
    }
    .round();
    let fat = (calorie * 0.28 / 9.0).round();
    let carbs = ((calorie - protein * 4.0 - fat * 9.0).max(0.0) / 4.0).round();
    Targets {
        calorie: calorie as i64,
        protein: protein as i64,
        carbs: carbs as i64,
        fat: fat as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn losing_targets_are_sane() {
        let t = calc_targets("female", 22, 168.0, 58.0, "moderate", "lose");
        assert!(t.calorie > 1400 && t.calorie < 2100, "{t:?}");
        assert!(t.protein > 100);
        assert!(t.carbs > 0 && t.fat > 0);
        // macros roughly add up to the calorie target
        let sum = t.protein * 4 + t.carbs * 4 + t.fat * 9;
        assert!((sum - t.calorie).abs() < 60, "{sum} vs {}", t.calorie);
    }

    #[test]
    fn weights_are_clamped() {
        let t = calc_targets("male", 30, 180.0, 80.0, "sedentary", "maintain");
        assert!(t.calorie >= 1200 && t.calorie <= 4500);
    }
}
