use wxrust::formatters::*;
use wxrust::models::{JDay, Set, Exercise, ExerciseWrapper, EBlock};

#[test]
fn test_format_weight() {
    assert_eq!(format_weight(100.0, false), "100");
    assert_eq!(format_weight(100.0, true), "220");  // 100 * 2.20462 ≈ 220, rounded
    assert_eq!(format_weight(45.5, false), "46");   // Rounded
}

#[test]
fn test_format_set() {
    let set = Set {
        w: Some(135.0),
        r: Some(5),
        s: Some(1),
        lb: Some(0.0),
        rpe: Some(8.0),
        c: Some("comment".to_string()),
        ..Default::default()
    };
    // Without color: "135 x 5 @8 comment"
    // But with color, it will have ANSI codes
    // For test, disable color
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted_no_color = format_set(&set);
    assert_eq!(formatted_no_color, "135 x 5 @8 comment");
}

#[test]
fn test_compress_sets_same_weight() {
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(135.0), r: Some(3), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let compressed = compress_sets(&sets);
    assert_eq!(compressed, vec!["135 x 5, 3".to_string()]);
}

#[test]
fn test_compress_sets_same_reps() {
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(145.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let compressed = compress_sets(&sets);
    assert_eq!(compressed, vec!["135, 145 x 5".to_string()]);
}

#[test]
fn test_compress_sets_no_compression() {
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(145.0), r: Some(3), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let compressed = compress_sets(&sets);
    assert_eq!(compressed.len(), 2);
    assert_eq!(compressed[0], "135 x 5");
    assert_eq!(compressed[1], "145 x 3");
}

#[test]
fn test_compress_sets_separated_same_weight() {
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(155.0), r: Some(3), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(135.0), r: Some(1), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let compressed = compress_sets(&sets);
    assert_eq!(compressed.len(), 3);
    assert_eq!(compressed[0], "135 x 5");
    assert_eq!(compressed[1], "155 x 3");
    assert_eq!(compressed[2], "135 x 1");
}



#[test]
fn test_summarize_workout() {
    let exercise = Exercise {
        id: "ex1".to_string(),
        name: "Squat".to_string(),
        ex_type: Some("strength".to_string()),
    };
    let ex_wrapper = ExerciseWrapper { exercise };
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
        Set { w: Some(145.0), r: Some(3), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    let eblock = EBlock {
        eid: "ex1".to_string(),
        sets,
    };
    let jday = JDay {
        log: "".to_string(),
        bw: Some(180.0),
        eblocks: vec![eblock],
        exercises: vec![ex_wrapper],
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let summary = summarize_workout(&jday);
    assert_eq!(summary, "#Squat  145x3");  // Max weight 145, max reps 3
}

#[test]
fn test_format_workout() {
    let exercise = Exercise {
        id: "ex1".to_string(),
        name: "Squat".to_string(),
        ex_type: Some("strength".to_string()),
    };
    let ex_wrapper = ExerciseWrapper { exercise };
    let sets = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    let eblock = EBlock {
        eid: "ex1".to_string(),
        sets,
    };
    let log = "Date: 2023-10-01\nEBLOCK:ex1\nSome text";
    let jday = JDay {
        log: log.to_string(),
        bw: Some(180.0),
        eblocks: vec![eblock],
        exercises: vec![ex_wrapper],
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_workout(&jday);
    assert!(formatted.contains("#Squat\n135 x 5"));
    assert!(formatted.contains("Date: 2023-10-01"));
    assert!(formatted.contains("Some text"));
}

#[test]
fn test_format_workout_multiple_eblocks() {
    let exercise1 = Exercise {
        id: "ex1".to_string(),
        name: "Squat".to_string(),
        ex_type: Some("strength".to_string()),
    };
    let exercise2 = Exercise {
        id: "ex2".to_string(),
        name: "Bench".to_string(),
        ex_type: Some("strength".to_string()),
    };
    let ex_wrapper1 = ExerciseWrapper { exercise: exercise1 };
    let ex_wrapper2 = ExerciseWrapper { exercise: exercise2 };
    let sets1 = vec![
        Set { w: Some(135.0), r: Some(5), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    let sets2 = vec![
        Set { w: Some(100.0), r: Some(8), s: Some(1), lb: Some(0.0), ..Default::default() },
    ];
    let eblock1 = EBlock {
        eid: "ex1".to_string(),
        sets: sets1,
    };
    let eblock2 = EBlock {
        eid: "ex2".to_string(),
        sets: sets2,
    };
    let log = "Date: 2023-10-01\nEBLOCK:ex1\nEBLOCK:ex2\nEnd";
    let jday = JDay {
        log: log.to_string(),
        bw: Some(180.0),
        eblocks: vec![eblock1, eblock2],
        exercises: vec![ex_wrapper1, ex_wrapper2],
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_workout(&jday);
    assert!(formatted.contains("Date: 2023-10-01"));
    assert!(formatted.contains("#Squat\n135 x 5"));
    assert!(formatted.contains("#Bench\n100 x 8"));
    assert!(formatted.contains("End"));
    // Ensure no duplication
    let squat_count = formatted.matches("#Squat").count();
    let bench_count = formatted.matches("#Bench").count();
    assert_eq!(squat_count, 1);
    assert_eq!(bench_count, 1);
}



