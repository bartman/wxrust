use wxrust::table::*;
use wxrust::models::{JDay, Set, Exercise, ExerciseWrapper, EBlock};

// ============================================================================
// 1RM Calculation Tests
// ============================================================================

#[test]
fn test_calculate_1rm_single_rep() {
    // 1RM of a single = weight
    assert_eq!(calculate_1rm(100.0, 1), 100.0);
    assert_eq!(calculate_1rm(225.0, 1), 225.0);
}

#[test]
fn test_calculate_1rm_brzycki() {
    // Brzycki: 1RM = weight * (36 / (37 - reps))
    // 100 * (36 / (37 - 5)) = 100 * (36/32) = 112.5
    let result = calculate_1rm(100.0, 5);
    assert!((result - 112.5).abs() < 0.1);

    // 135 * (36 / (37 - 3)) = 135 * (36/34) = 142.94
    let result = calculate_1rm(135.0, 3);
    assert!((result - 142.94).abs() < 0.1);
}

#[test]
fn test_calculate_1rm_edge_cases() {
    assert_eq!(calculate_1rm(0.0, 5), 0.0);
    assert_eq!(calculate_1rm(100.0, 0), 0.0);
    assert_eq!(calculate_1rm(-10.0, 5), 0.0);
}

#[test]
fn test_calculate_1rm_high_reps() {
    // Formula breaks at 37+ reps, should return rough estimate
    let result = calculate_1rm(100.0, 40);
    assert!(result > 0.0); // Should be positive
}

#[test]
fn test_weight_from_1rm_single_rep() {
    assert_eq!(weight_from_1rm(100.0, 1), 100.0);
    assert_eq!(weight_from_1rm(225.0, 1), 225.0);
}

#[test]
fn test_weight_from_1rm_brzycki() {
    // Inverse of calculate_1rm
    let onerm = 112.5;
    let weight = weight_from_1rm(onerm, 5);
    assert!((weight - 100.0).abs() < 0.1);
}

#[test]
fn test_weight_from_1rm_edge_cases() {
    assert_eq!(weight_from_1rm(0.0, 5), 0.0);
    assert_eq!(weight_from_1rm(100.0, 0), 0.0);
}

#[test]
fn test_1rm_round_trip() {
    // Calculate 1RM then convert back to weight
    let weight = 135.0;
    let reps = 5;
    let onerm = calculate_1rm(weight, reps);
    let recovered = weight_from_1rm(onerm, reps);
    assert!((recovered - weight).abs() < 0.01);
}

// ============================================================================
// Date Argument Detection Tests
// ============================================================================

#[test]
fn test_is_date_arg_year() {
    assert!(is_date_arg("2025"));
    assert!(is_date_arg("2024"));
    assert!(is_date_arg("1999"));
}

#[test]
fn test_is_date_arg_year_month() {
    assert!(is_date_arg("2025-01"));
    assert!(is_date_arg("2025.01"));
    assert!(is_date_arg("202501"));
    assert!(is_date_arg("2025/06"));
}

#[test]
fn test_is_date_arg_full_date() {
    assert!(is_date_arg("2025-01-15"));
    assert!(is_date_arg("2025.01.15"));
    assert!(is_date_arg("20250115"));
    assert!(is_date_arg("2025/01/15"));
}

#[test]
fn test_is_date_arg_not_dates() {
    assert!(!is_date_arg("deadlift"));
    assert!(!is_date_arg("squat"));
    assert!(!is_date_arg("#bp"));
    assert!(!is_date_arg("bench press"));
    assert!(!is_date_arg("dl"));
    assert!(!is_date_arg("ohp"));
}

#[test]
fn test_is_date_arg_edge_cases() {
    // Partial dates that don't parse
    assert!(!is_date_arg("202")); // Too short
    assert!(!is_date_arg("12345")); // Invalid year format
}

// ============================================================================
// Argument Parsing Tests
// ============================================================================

#[test]
fn test_parse_arguments_mixed() {
    let args: Vec<String> = vec![
        "2025".to_string(),
        "deadlift".to_string(),
        "2025-06".to_string(),
        "squat".to_string(),
    ];

    let (dates, filters) = wxrust::table::parse_date_and_filter_arguments(&args);

    assert_eq!(dates, vec!["2025", "2025-06"]);
    assert_eq!(filters, vec!["deadlift", "squat"]);
}

#[test]
fn test_parse_arguments_only_dates() {
    let args: Vec<String> = vec![
        "2025".to_string(),
        "2025-01".to_string(),
        "2025-01-15".to_string(),
    ];

    let (dates, filters) = wxrust::table::parse_date_and_filter_arguments(&args);

    assert_eq!(dates.len(), 3);
    assert!(filters.is_empty());
}

#[test]
fn test_parse_arguments_only_filters() {
    let args: Vec<String> = vec![
        "deadlift".to_string(),
        "squat".to_string(),
        "bench".to_string(),
    ];

    let (dates, filters) = wxrust::table::parse_date_and_filter_arguments(&args);

    assert!(dates.is_empty());
    assert_eq!(filters.len(), 3);
}

#[test]
fn test_parse_arguments_empty() {
    let args: Vec<String> = vec![];

    let (dates, filters) = wxrust::table::parse_date_and_filter_arguments(&args);

    assert!(dates.is_empty());
    assert!(filters.is_empty());
}

// ============================================================================
// TableState Tests
// ============================================================================

#[test]
fn test_table_state_new() {
    let state = TableState::new();
    assert!(state.records.is_empty());
    assert_eq!(state.max_lift_width, 0);
}

#[test]
fn test_table_state_process_set_first_record() {
    let mut state = TableState::new();

    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);

    assert_eq!(state.records.len(), 1);
    assert_eq!(state.max_lift_width, 8); // "Deadlift" length
    assert_eq!(state.records[0].exercise_name, "Deadlift");
    assert_eq!(state.records[0].best_weight, 100.0);
    assert_eq!(state.records[0].best_reps, 5);
}

#[test]
fn test_table_state_process_set_not_pr() {
    let mut state = TableState::new();

    // First set creates a record
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);

    // Same weight/reps - not a PR
    state.process_set("2025-01-02", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1); // No new record
}

#[test]
fn test_table_state_process_set_new_pr() {
    let mut state = TableState::new();

    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);

    // Better weight for same reps - new PR
    state.process_set("2025-01-03", "Deadlift", Some(80.0), 110.0, 5);
    assert_eq!(state.records.len(), 2);
}

#[test]
fn test_table_state_process_set_different_rep_range() {
    let mut state = TableState::new();

    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);

    // Different rep range - new PR
    state.process_set("2025-01-04", "Deadlift", Some(80.0), 120.0, 3);
    assert_eq!(state.records.len(), 2);
}

#[test]
fn test_table_state_process_set_invalid_input() {
    let mut state = TableState::new();

    // Zero weight
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 0.0, 5);
    assert!(state.records.is_empty());

    // Zero reps
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 0);
    assert!(state.records.is_empty());
}

#[test]
fn test_table_state_process_set_same_day_same_reps_update() {
    let mut state = TableState::new();

    // First set for 5 reps
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);
    assert_eq!(state.records[0].best_weight, 100.0);
    assert_eq!(state.records[0].best_reps, 5);

    // Better weight for same reps on same day - should update existing record
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 110.0, 5);
    assert_eq!(state.records.len(), 1); // Still only 1 record
    assert_eq!(state.records[0].best_weight, 110.0); // Updated weight
    assert_eq!(state.records[0].best_reps, 5); // Same reps
}

// ============================================================================
// Workout Processing Tests
// ============================================================================

fn make_test_jday(exercise_name: &str, sets: Vec<(f32, u32)>) -> JDay {
    let exercise = Exercise {
        id: "ex1".to_string(),
        name: exercise_name.to_string(),
        ex_type: Some("strength".to_string()),
    };
    let ex_wrapper = ExerciseWrapper { exercise };

    let set_structs: Vec<Set> = sets
        .into_iter()
        .map(|(w, r)| Set {
            w: Some(w),
            r: Some(r),
            s: Some(1),
            lb: Some(0.0),
            ..Default::default()
        })
        .collect();

    let eblock = EBlock {
        eid: "ex1".to_string(),
        sets: set_structs,
    };

    JDay {
        log: "".to_string(),
        bw: Some(80.0),
        eblocks: vec![eblock],
        exercises: vec![ex_wrapper],
    }
}

#[test]
fn test_process_workout_no_filters() {
    let jday = make_test_jday("Deadlift", vec![(100.0, 5), (120.0, 3)]);
    let mut state = TableState::new();
    let filters: Vec<String> = vec![];

    process_workout(&jday, "2025-01-01", &filters, &mut state);

    // Both sets should create records (different rep ranges)
    assert_eq!(state.records.len(), 2);
}

#[test]
fn test_process_workout_with_matching_filter() {
    let jday = make_test_jday("Deadlift", vec![(100.0, 5)]);
    let mut state = TableState::new();
    let filters: Vec<String> = vec!["dead".to_string()];

    process_workout(&jday, "2025-01-01", &filters, &mut state);

    assert_eq!(state.records.len(), 1);
    assert_eq!(state.records[0].exercise_name, "Deadlift");
}

#[test]
fn test_process_workout_with_non_matching_filter() {
    let jday = make_test_jday("Deadlift", vec![(100.0, 5)]);
    let mut state = TableState::new();
    let filters: Vec<String> = vec!["squat".to_string()];

    process_workout(&jday, "2025-01-01", &filters, &mut state);

    assert!(state.records.is_empty());
}

#[test]
fn test_process_workout_case_insensitive_filter() {
    let jday = make_test_jday("Deadlift", vec![(100.0, 5)]);
    let mut state = TableState::new();
    let filters: Vec<String> = vec!["DEAD".to_string()];

    process_workout(&jday, "2025-01-01", &filters, &mut state);

    assert_eq!(state.records.len(), 1);
}

#[test]
fn test_process_workout_multiple_filters_or() {
    let jday = make_test_jday("Deadlift", vec![(100.0, 5)]);
    let mut state = TableState::new();
    let filters: Vec<String> = vec!["squat".to_string(), "dead".to_string()];

    process_workout(&jday, "2025-01-01", &filters, &mut state);

    // Should match because "dead" matches "Deadlift"
    assert_eq!(state.records.len(), 1);
}

// ============================================================================
// Format Table Tests
// ============================================================================

#[test]
fn test_format_table_empty() {
    let state = TableState::new();
    let filters: Vec<String> = vec![];

    let output = format_table(&state, &filters, true);

    assert_eq!(output, "No records found.");
}

#[test]
fn test_format_table_with_records() {
    let mut state = TableState::new();
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);

    let filters: Vec<String> = vec!["dead".to_string()];

    // Disable color for testing
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }

    let output = format_table(&state, &filters, true);

    assert!(output.contains("dead")); // Filter name in header
    assert!(output.contains("There were 1 records of dead")); // Count in header
    assert!(output.contains("2025-01-01")); // Date in output
    assert!(output.contains("Deadlift")); // Exercise name
}

#[test]
fn test_format_table_header_row() {
    let mut state = TableState::new();
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);

    let filters: Vec<String> = vec![];

    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }

    let output = format_table(&state, &filters, true);

    // Check header contains expected columns
    assert!(output.contains("date"));
    assert!(output.contains("days"));
    assert!(output.contains("BW"));
    assert!(output.contains("lift"));
    assert!(output.contains("1RM"));
}

#[test]
fn test_format_table_best_weight_row() {
    let mut state = TableState::new();
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);

    let filters: Vec<String> = vec![];

    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }

    let output = format_table(&state, &filters, true);

    assert!(output.contains("best weight lifted"));
}

#[test]
fn test_gradient_color() {
    // Recent records get bright yellow
    assert_eq!(get_gradient_color(100, 100, 5), BRIGHT_YELLOW);

    // Older records use gradient
    let col = get_gradient_color(0, 100, 100);
    assert_eq!(col, GRADIENT[0]);

    let col = get_gradient_color(99, 100, 100);
    assert_eq!(col, GRADIENT[GRADIENT.len() - 1]);
}

#[test]
fn test_table_state_process_set() {
    let mut state = TableState::new();

    // First set - should create a record
    state.process_set("2025-01-01", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);
    assert_eq!(state.max_lift_width, 8);

    // Same weight/reps but different date - not a PR
    state.process_set("2025-01-02", "Deadlift", Some(80.0), 100.0, 5);
    assert_eq!(state.records.len(), 1);

    // Better 1RM for same rep range - should create a record
    state.process_set("2025-01-03", "Deadlift", Some(80.0), 110.0, 5);
    assert_eq!(state.records.len(), 2);

    // Different rep range - should create a record
    state.process_set("2025-01-04", "Deadlift", Some(80.0), 120.0, 3);
    assert_eq!(state.records.len(), 3);
}
