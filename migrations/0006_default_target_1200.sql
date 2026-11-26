-- New/unonboarded users start at 1200 kcal; completed profiles keep their targets.
UPDATE users
SET calorie_target = 1200, protein_target = 60, carb_target = 130, fat_target = 40
WHERE onboarded = 0;
