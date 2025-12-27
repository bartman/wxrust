use wxrust::parsers::*;
use wxrust::formatters::*;
use wxrust::models::{JDay, Set, Exercise, ExerciseWrapper, EBlock};

#[test]
fn test_parse_workout_simple() {
    let text = "2025-01-21
@ 215 bw
cutting, fasted
#lat-pulldown
175 x 10 x 3
#cable-low-row
175 x 10 x 3
";

    let jday = parse_workout(text).unwrap();
    assert_eq!(jday.bw, Some(215.0));
    assert_eq!(jday.exercises.len(), 2);
    assert_eq!(jday.exercises[0].exercise.name, "lat-pulldown");
    assert_eq!(jday.exercises[1].exercise.name, "cable-low-row");
    assert_eq!(jday.eblocks.len(), 2);
    assert_eq!(jday.eblocks[0].sets.len(), 1); // compressed
    assert_eq!(jday.eblocks[0].sets[0].w, Some(175.0));
    assert_eq!(jday.eblocks[0].sets[0].r, Some(10));
    assert_eq!(jday.eblocks[0].sets[0].s, Some(3));
}

#[test]
fn test_round_trip() {
    // Create a JDay
    let exercise1 = Exercise {
        id: "lat-pulldown".to_string(),
        name: "lat-pulldown".to_string(),
        ex_type: None,
    };
    let ex_wrapper1 = ExerciseWrapper { exercise: exercise1 };
    let sets1 = vec![
        Set { w: Some(175.0), r: Some(10), s: Some(3), lb: Some(0.0), rpe: None, c: None, set_type: Some(0) },
    ];
    let eblock1 = EBlock {
        eid: "lat-pulldown".to_string(),
        sets: sets1,
    };
    let exercise2 = Exercise {
        id: "cable-low-row".to_string(),
        name: "cable-low-row".to_string(),
        ex_type: None,
    };
    let ex_wrapper2 = ExerciseWrapper { exercise: exercise2 };
    let sets2 = vec![
        Set { w: Some(175.0), r: Some(10), s: Some(3), lb: Some(0.0), rpe: None, c: None, set_type: Some(0) },
    ];
    let eblock2 = EBlock {
        eid: "cable-low-row".to_string(),
        sets: sets2,
    };
    let log = "cutting, fasted\nEBLOCK:lat-pulldown\nEBLOCK:cable-low-row\n";
    let original_jday = JDay {
        log: log.to_string(),
        bw: Some(215.0),
        eblocks: vec![eblock1, eblock2],
        exercises: vec![ex_wrapper1, ex_wrapper2],
    };

    // Format to full text (simulate render_workout without color)
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let user = wxrust::models::User { usekg: Some(1) };
    let full_formatted_text = format!("2025-01-21\n@ 215 bw\n{}", format_workout_no_color(&original_jday));


    // Parse back
    let parsed_jday = parse_workout(&full_formatted_text).unwrap();

    // Format again
    let reformatted_full_text = format!("2025-01-21\n@ 215 bw\n{}", format_workout_no_color(&parsed_jday));

    // Should match
    assert_eq!(full_formatted_text, reformatted_full_text);
}
