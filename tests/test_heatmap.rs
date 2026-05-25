use wxrust::heatmap::{compute_metric, Metric};
use wxrust::models::{JDay, Exercise, EBlock, Set};

#[test]
fn test_compute_metric_sets() {
    let jday = create_test_jday();
    let metric = Metric::Sets;
    let filters = Vec::new();

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 6.0); // 2 + 4
}

#[test]
fn test_compute_metric_reps() {
    let jday = create_test_jday();
    let metric = Metric::Reps;
    let filters = Vec::new();

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 30.0); // 5*2 + 5*4
}

#[test]
fn test_compute_metric_volume() {
    let jday = create_test_jday();
    let metric = Metric::Volume;
    let filters = Vec::new();

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 3000.0); // 100*5*2 + 100*5*4
}

#[test]
fn test_compute_metric_weight() {
    let jday = create_test_jday();
    let metric = Metric::Weight;
    let filters = Vec::new();

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 100.0);
}

#[test]
fn test_compute_metric_onerm() {
    let jday = create_test_jday();
    let metric = Metric::OneRm;
    let filters = Vec::new();

    let result = compute_metric(&jday, metric, &filters);
    // 1RM for 100x5 = 112.5
    assert!((result - 112.5).abs() < 0.1);
}

#[test]
fn test_compute_metric_with_filters() {
    let jday = create_test_jday();
    let metric = Metric::Sets;
    let filters = vec!["Deadlift".to_string()];

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 4.0); // Only deadlift sets
}

#[test]
fn test_compute_metric_no_matches() {
    let jday = create_test_jday();
    let metric = Metric::Sets;
    let filters = vec!["NonExistent".to_string()];

    let result = compute_metric(&jday, metric, &filters);
    assert_eq!(result, 0.0);
}

fn create_test_jday() -> JDay {
    let exercises = vec![
        wxrust::models::ExerciseWrapper {
            exercise: Exercise {
                id: "ex1".to_string(),
                name: "Bench Press".to_string(),
                ex_type: Some("Chest".to_string()),
            },
        },
        wxrust::models::ExerciseWrapper {
            exercise: Exercise {
                id: "ex2".to_string(),
                name: "Deadlift".to_string(),
                ex_type: Some("Back".to_string()),
            },
        },
    ];

    let eblocks = vec![
        EBlock {
            eid: "ex1".to_string(),
            sets: vec![
                Set {
                    w: Some(100.0),
                    r: Some(5),
                    s: Some(2),
                    ..Default::default()
                },
            ],
        },
        EBlock {
            eid: "ex2".to_string(),
            sets: vec![
                Set {
                    w: Some(100.0),
                    r: Some(5),
                    s: Some(4),
                    ..Default::default()
                },
            ],
        },
    ];

    JDay {
        exercises,
        eblocks,
        bw: Some(80.0),
        log: "".to_string(),
    }
}
