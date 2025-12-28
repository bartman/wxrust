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
    let _user = wxrust::models::User { usekg: Some(1) };
    let full_formatted_text = format_workout_no_color("2025-01-21", &original_jday, true);


    // Parse back
    let parsed_jday = parse_workout(&full_formatted_text).unwrap();

    // Format again
    let reformatted_full_text = format_workout_no_color("2025-01-21", &parsed_jday, true);

    // Should match
    assert_eq!(full_formatted_text, reformatted_full_text);
}

#[test]
fn test_parse_cache_text() {
    let cache_text = r#"2025-12-26
@ 105 bw
531 squat C26 W2
TM: 495
#safety-squat #sq #SQ
165 x 10
255 x 5
305, 360, 415 x 3
455 x 6
// skip: 360 x 5 AMRAP
// https://fivethreeone.app/calculator?program=NU-LTsMwEPyVas6ryI5jle6tRTwkWgriiHpISwpFwKEx6iHyv3dsJ5fdmd2ZHe2AFmoEe.i7vXHi3EKahRc-tzvBATrgA9oIOgoGXEjqKAXYCbgJmEjTEXpsf-pO8DmiKHhKl9Y5agNlAJ55wLFv8-Ah1xdoOP8n-VfSn1KxOd5TLbCG8Ww.q.sydIU1ibH7Qq0xRVSN1qouC1PNJ.DzaVM2DP0m9.y-KfcPivvTuQ.zty7M1m0fIHhMm7tsD-xb0EN9pGWZFitaKLotXwhex3fiFQ__"#;

    // Parse the cache text
    let parsed_jday = parse_workout(cache_text).unwrap();

    // The parsed JDay should have the correct structure
    assert_eq!(parsed_jday.bw, Some(105.0));
    assert_eq!(parsed_jday.exercises.len(), 1);
    assert_eq!(parsed_jday.exercises[0].exercise.name, "safety-squat #sq #SQ");
    assert_eq!(parsed_jday.eblocks.len(), 1);
    assert_eq!(parsed_jday.eblocks[0].eid, "safety-squat #sq #SQ");
    assert_eq!(parsed_jday.eblocks[0].sets.len(), 5); // 165x10, 255x5, 305x3, 360x3, 415x3, 455x6

    // Check the log
    let expected_log = "531 squat C26 W2\nTM: 495\nEBLOCK:safety-squat #sq #SQ\n// skip: 360 x 5 AMRAP\n// https://fivethreeone.app/calculator?program=NU-LTsMwEPyVas6ryI5jle6tRTwkWgriiHpISwpFwKEx6iHyv3dsJ5fdmd2ZHe2AFmoEe.i7vXHi3EKahRc-tzvBATrgA9oIOgoGXEjqKAXYCbgJmEjTEXpsf-pO8DmiKHhKl9Y5agNlAJ55wLFv8-Ah1xdoOP8n-VfSn1KxOd5TLbCG8Ww.q.sydIU1ibH7Qq0xRVSN1qouC1PNJ.DzaVM2DP0m9.y-KfcPivvTuQ.zty7M1m0fIHhMm7tsD-xb0EN9pGWZFitaKLotXwhex3fiFQ__";
    assert_eq!(parsed_jday.log, expected_log);

    // Format back and check it matches the input
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let reformatted = format_workout_no_color("2025-12-26", &parsed_jday, true);
    assert_eq!(reformatted, cache_text);
}
