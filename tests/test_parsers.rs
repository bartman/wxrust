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
        Set { w: Some(175.0), r: Some(10), s: Some(3), lb: Some(0.0), rpe: None, c: None, set_type: Some(0), usebw: None },
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
        Set { w: Some(175.0), r: Some(10), s: Some(3), lb: Some(0.0), rpe: None, c: None, set_type: Some(0), usebw: None },
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
    let full_formatted_text = format_workout_for_cache("2025-01-21", &original_jday);


    // Parse back
    let parsed_jday = parse_workout(&full_formatted_text).unwrap();

    // Format again
    let reformatted_full_text = format_workout_for_cache("2025-01-21", &parsed_jday);

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
// https://fivethreeone.app/calculator?program=NU-LTsMwEPyVas6ryI5jle6tRTwkWgriiHpISwpFwKEx6iHyv3dsJ5fdmd2ZHe2AFmoEe.i7vXHi3EKahRc-tzvBATrgA9oIOgoGXEjqKAXYCbgJmEjTEXpsf-pO8DmiKHhKl9Y5agNlAJ55wLFv8-Ah1xdoOP8n-VfSn1KxOd5TLbCG8Ww.q.sydIU1ibH7Qq0xRVSN1qouC1PNJ.DzaVM2DP0m9.y-KfcPivvTuQ.zty7M1m0fIHhMm7tsD-xb0EN9pGWZFitaKLotXwhex3fiFQ__"#.to_string() + "\n";

    // Parse the cache text
    let parsed_jday = parse_workout(&cache_text).unwrap();

    // The parsed JDay should have the correct structure
    assert_eq!(parsed_jday.bw, Some(105.0));
    assert_eq!(parsed_jday.exercises.len(), 1);
    assert_eq!(parsed_jday.exercises[0].exercise.name, "safety-squat #sq #SQ");
    assert_eq!(parsed_jday.eblocks.len(), 1);
    assert_eq!(parsed_jday.eblocks[0].eid, "safety-squat #sq #SQ");
    assert_eq!(parsed_jday.eblocks[0].sets.len(), 6); // 165x10, 255x5, 305x3, 360x3, 415x3, 455x6

    // Check the log
    let expected_log = "531 squat C26 W2\nTM: 495\nEBLOCK:safety-squat #sq #SQ\n// skip: 360 x 5 AMRAP\n// https://fivethreeone.app/calculator?program=NU-LTsMwEPyVas6ryI5jle6tRTwkWgriiHpISwpFwKEx6iHyv3dsJ5fdmd2ZHe2AFmoEe.i7vXHi3EKahRc-tzvBATrgA9oIOgoGXEjqKAXYCbgJmEjTEXpsf-pO8DmiKHhKl9Y5agNlAJ55wLFv8-Ah1xdoOP8n-VfSn1KxOd5TLbCG8Ww.q.sydIU1ibH7Qq0xRVSN1qouC1PNJ.DzaVM2DP0m9.y-KfcPivvTuQ.zty7M1m0fIHhMm7tsD-xb0EN9pGWZFitaKLotXwhex3fiFQ__\n";
    assert_eq!(parsed_jday.log, expected_log);

    // Format back and check it matches the input
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let reformatted = format_workout_no_color("2025-12-26", &parsed_jday, true);
    assert_eq!(reformatted, cache_text);
}

#[test]
fn test_parse_2025_12_12() {
    let cache_text = r#"2025-12-12
@ 102.9650 kg bw
531 squat C25 W3
TM: 485
#safety-box-squat #sq
195 kg x 10
245 kg x 5
295 kg x 3
375 kg x 5
425 kg x 3
472 kg x 7 hard, but rewarding
375 kg x 5 AMRAP
// https://fivethreeone.app/calculator?program=NU-LbsJADPwVNGcr2s3GPHxrK2ilAi3iiHpI26SA2h7IIg7R-jve3eRiz9gzHrlHDTGET8jBzi05N6VqzsTT6oPwBenxDakIjQp63JSUgTKwI3AjMEFNLaStf7uG8DOgQHiNl9YpagNZMGGrB5z2tzR8TvUd4i-XqD9G-SkWm.JZ1QRrNF4bJ3WZhy6zKjLtnKk1JouKwVqUeWGK2Qg4nTZ5o6Fn5az9L.b.Q7A6XTo-2Td.sq47D8JL3CyT3evfhA7CQS0PcfGoFhU95S8Iu.GdcAc_"#.to_string() + "\n";

    // Parse the cache text
    let parsed_jday = parse_workout(&cache_text).unwrap();

    // The parsed JDay should have the correct structure
    assert_eq!(parsed_jday.bw, Some(102.9650));
    assert_eq!(parsed_jday.exercises.len(), 1);
    assert_eq!(parsed_jday.exercises[0].exercise.name, "safety-box-squat #sq");
    assert_eq!(parsed_jday.eblocks.len(), 1);
    assert_eq!(parsed_jday.eblocks[0].eid, "safety-box-squat #sq");
    assert_eq!(parsed_jday.eblocks[0].sets.len(), 7); // 195x10, 245x5, 295x3, 375x5, 425x3, 472x7, 375x5

    // Check some sets
    assert_eq!(parsed_jday.eblocks[0].sets[0].w, Some(195.0));
    assert_eq!(parsed_jday.eblocks[0].sets[0].r, Some(10));
    assert_eq!(parsed_jday.eblocks[0].sets[0].s, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[5].w, Some(472.0));
    assert_eq!(parsed_jday.eblocks[0].sets[5].r, Some(7));
    assert_eq!(parsed_jday.eblocks[0].sets[5].s, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[5].c, Some("hard, but rewarding".to_string()));
    assert_eq!(parsed_jday.eblocks[0].sets[6].w, Some(375.0));
    assert_eq!(parsed_jday.eblocks[0].sets[6].r, Some(5));
    assert_eq!(parsed_jday.eblocks[0].sets[6].s, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[6].c, Some("AMRAP".to_string()));

    // Format back and check it matches the input
    let reformatted = format_workout_for_cache("2025-12-12", &parsed_jday);
    assert_eq!(reformatted, cache_text);
}

#[test]
fn test_parse_2025_12_18() {
    let cache_text = r#"2025-12-18
@ 102.9650 kg bw
531 ohp C26 W1
TM: 183
#cambered-ohp #ohp
70 kg x 10
90 kg x 5
110 kg x 3
119 kg, 138 kg x 5
156 kg x 7
120 kg x 10 AMRAP
shoulders need more work
#dumbbell-side-raise
5 kg, 10 kg, 15 kg x 10
#weight-plate-front-raise
25 kg x 10 x 3
// https://fivethreeone.app/calculator?program=NU-LTsMwEPyVas6ryI5jle6tRTwkWgriiHpISwpFwKEx6iHyv3dsJ5fdmd2ZHe2AFmoEe.i7vXHi3EKahRc-tzvBATrgA9oIOgoGXEjqKAXYCbgJmEjTEXpsf-pO8DmiKHhKl9Y5agNlAJ55wLFv8-Ah1xdoOP8n-VfSn1KxOd5TLbCG8Ww.q.sydIU1ibH7Qq0xRVSN1qouC1PNJ.DzaVM2DP0m9.y-KfcPivvTuQ.zty7M1m0fIHhMm7tsD-xb0EN9pGWZFitaKLotXwhex3fiFQ__"#.to_string() + "\n";

    // Parse the cache text
    let parsed_jday = parse_workout(&cache_text).unwrap();

    // The parsed JDay should have the correct structure
    assert_eq!(parsed_jday.bw, Some(102.9650));
    assert_eq!(parsed_jday.exercises.len(), 3);
    assert_eq!(parsed_jday.exercises[0].exercise.name, "cambered-ohp #ohp");
    assert_eq!(parsed_jday.exercises[1].exercise.name, "dumbbell-side-raise");
    assert_eq!(parsed_jday.exercises[2].exercise.name, "weight-plate-front-raise");
    assert_eq!(parsed_jday.eblocks.len(), 3);
    assert_eq!(parsed_jday.eblocks[0].eid, "cambered-ohp #ohp");
    assert_eq!(parsed_jday.eblocks[0].sets.len(), 7); // 70x10, 90x5, 110x3, 119x5, 138x5, 156x7, 120x10
    assert_eq!(parsed_jday.eblocks[1].eid, "dumbbell-side-raise");
    assert_eq!(parsed_jday.eblocks[1].sets.len(), 3); // 5x10, 10x10, 15x10
    assert_eq!(parsed_jday.eblocks[2].eid, "weight-plate-front-raise");
    assert_eq!(parsed_jday.eblocks[2].sets.len(), 1); // 25x10x3

    // Check some sets
    assert_eq!(parsed_jday.eblocks[0].sets[0].w, Some(70.0));
    assert_eq!(parsed_jday.eblocks[0].sets[0].r, Some(10));
    assert_eq!(parsed_jday.eblocks[0].sets[6].c, Some("AMRAP".to_string()));
    assert_eq!(parsed_jday.eblocks[1].sets[0].w, Some(5.0));
    assert_eq!(parsed_jday.eblocks[1].sets[0].r, Some(10));
    assert_eq!(parsed_jday.eblocks[2].sets[0].w, Some(25.0));
    assert_eq!(parsed_jday.eblocks[2].sets[0].r, Some(10));
    assert_eq!(parsed_jday.eblocks[2].sets[0].s, Some(3));

    // Format back and check it matches the input
    let reformatted = format_workout_for_cache("2025-12-18", &parsed_jday);
    assert_eq!(reformatted, cache_text);
}

#[test]
fn test_parse_set_line_examples() {
    // 405               - ( Set { w=405, r=1, s=1, c="" } )
    let sets = parse_set_line("405").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(1));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);

    // 405 cccc          - ( Set { w=405, r=1, s=1, c="cccc" } )
    let sets = parse_set_line("405 cccc").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(1));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, Some("cccc".to_string()));

    // 405 x 2           - ( Set { w=405, r=2, s=1, c="" } )
    let sets = parse_set_line("405 x 2").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);

    // 405 x 2 x 3       - ( Set { w=405, r=2, s=3, c="" } )
    let sets = parse_set_line("405 x 2 x 3").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(3));
    assert_eq!(sets[0].c, None);

    // 405 x 2, 3        - ( Set { w=405, r=2, s=1, c="" }, Set { w=405, r=3, s=1, c="" } )
    let sets = parse_set_line("405 x 2, 3").unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);
    assert_eq!(sets[1].w, Some(405.0));
    assert_eq!(sets[1].r, Some(3));
    assert_eq!(sets[1].s, Some(1));
    assert_eq!(sets[1].c, None);

    // 405 x 2, 3 cccc   - ( Set { w=405, r=2, s=1, c="" }, Set { w=405, r=3, s=1, c="cccc" } )
    let sets = parse_set_line("405 x 2, 3 cccc").unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);
    assert_eq!(sets[1].w, Some(405.0));
    assert_eq!(sets[1].r, Some(3));
    assert_eq!(sets[1].s, Some(1));
    assert_eq!(sets[1].c, Some("cccc".to_string()));

    // 405, 406 x 2      - ( Set { w=405, r=2, s=1, c="" }, Set { w=406, r=2, s=1, c="" } )
    let sets = parse_set_line("405, 406 x 2").unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);
    assert_eq!(sets[1].w, Some(406.0));
    assert_eq!(sets[1].r, Some(2));
    assert_eq!(sets[1].s, Some(1));
    assert_eq!(sets[1].c, None);

    // 405, 406 x 2 cccc - ( Set { w=405, r=2, s=1, c="" }, Set { w=406, r=2, s=1, c="cccc" } )
    let sets = parse_set_line("405, 406 x 2 cccc").unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(2));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].c, None);
    assert_eq!(sets[1].w, Some(406.0));
    assert_eq!(sets[1].r, Some(2));
    assert_eq!(sets[1].s, Some(1));
    assert_eq!(sets[1].c, Some("cccc".to_string()));
}

#[test]
fn test_parse_2025_10_31() {
    let cache_text = r#"2025-10-31
@ 100.6980 kg bw
531 squat C23 W3
TM: 465
#safety-box-squat #sq
135 kg x 10
235 kg x 5
285 kg x 3
350 kg x 5
405 kg x 3
445 kg x 1, 3
350 kg x 5 AMRAP
// https://fivethreeone.app/calculator?program=NU-LTsNADPyVas5W5M3GLfhWEFCp5SWOqIcUUiiCHpqteoj23-HuJhd7xp7xyANaKBN20He3uCLvmZq5kAhvCR-QAZ-QhtCZYMDFSB2pADcBPwGOZtpD9.1v3xG.RhQJ63Rpk6MeoddCeLID3vpzHj7k.gINp3PSfyf9IRWX48XUBMcWb02yui5DX1iTmHUp1DEXUTVaq7osuFpMQPJpLhsL-TEu1v9S7hGK.8OpD7O3Lsw2bR9AWKXNXbYH.5vQQyWaZZkWN2Yx0W35gvA6vhP-AQ__"#.to_string() + "\n";

    // Parse the cache text
    let parsed_jday = parse_workout(&cache_text).unwrap();

    // The parsed JDay should have the correct structure
    assert_eq!(parsed_jday.bw, Some(100.6980));
    assert_eq!(parsed_jday.exercises.len(), 1);
    assert_eq!(parsed_jday.exercises[0].exercise.name, "safety-box-squat #sq");
    assert_eq!(parsed_jday.eblocks.len(), 1);
    assert_eq!(parsed_jday.eblocks[0].eid, "safety-box-squat #sq");
    assert_eq!(parsed_jday.eblocks[0].sets.len(), 8); // 135x10, 235x5, 285x3, 350x5, 405x3, 445x1, 445x3, 350x5

    // Check the compressed reps sets
    assert_eq!(parsed_jday.eblocks[0].sets[5].w, Some(445.0));
    assert_eq!(parsed_jday.eblocks[0].sets[5].r, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[5].s, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[6].w, Some(445.0));
    assert_eq!(parsed_jday.eblocks[0].sets[6].r, Some(3));
    assert_eq!(parsed_jday.eblocks[0].sets[6].s, Some(1));
    assert_eq!(parsed_jday.eblocks[0].sets[7].c, Some("AMRAP".to_string()));

    // Format back and check it matches the input
    let reformatted = format_workout_for_cache("2025-10-31", &parsed_jday);
    assert_eq!(reformatted, cache_text);
}

#[test]
fn test_parse_rpe() {
    // Test RPE parsing
    let sets = parse_set_line("405 x 5 @8").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].r, Some(5));
    assert_eq!(sets[0].s, Some(1));
    assert_eq!(sets[0].rpe, Some(8.0));
    assert_eq!(sets[0].c, None);

    let sets = parse_set_line("405 x 5 @7.5 hard").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].rpe, Some(7.5));
    assert_eq!(sets[0].c, Some("hard".to_string()));
}

#[test]
fn test_parse_bw_exercises() {
    // Test BW exercises
    let sets = parse_set_line("BW x 10").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(0.0));
    assert_eq!(sets[0].r, Some(10));
    assert_eq!(sets[0].usebw, Some(1));

    let sets = parse_set_line("BW+ 20 x 5").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(20.0));
    assert_eq!(sets[0].r, Some(5));
    assert_eq!(sets[0].usebw, Some(1));

    let sets = parse_set_line("BW- 10 x 5").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(10.0));
    assert_eq!(sets[0].r, Some(5));
    assert_eq!(sets[0].usebw, Some(-1));
}

#[test]
fn test_parse_units() {
    // Test units
    let sets = parse_set_line("405 lb x 5").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(405.0));
    assert_eq!(sets[0].lb, Some(1.0));

    let sets = parse_set_line("180 kg x 5").unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].w, Some(180.0));
    assert_eq!(sets[0].lb, Some(0.0));
}

#[test]
fn test_format_rpe() {
    let set = Set {
        w: Some(405.0),
        r: Some(5),
        s: Some(1),
        lb: Some(0.0),
        rpe: Some(8.0),
        c: None,
        set_type: Some(0),
        usebw: None,
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_set(&set);
    assert_eq!(formatted, "405 x 5 @8");
}

#[test]
fn test_format_bw() {
    let set = Set {
        w: Some(0.0),
        r: Some(10),
        s: Some(1),
        lb: Some(0.0),
        rpe: None,
        c: None,
        set_type: Some(0),
        usebw: Some(1),
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_set(&set);
    assert_eq!(formatted, "BW x 10");

    let set = Set {
        w: Some(20.0),
        r: Some(5),
        s: Some(1),
        lb: Some(0.0),
        rpe: None,
        c: None,
        set_type: Some(0),
        usebw: Some(1),
    };
    let formatted = format_set(&set);
    assert_eq!(formatted, "BW+20 x 5");

    let set = Set {
        w: Some(10.0),
        r: Some(5),
        s: Some(1),
        lb: Some(0.0),
        rpe: None,
        c: None,
        set_type: Some(0),
        usebw: Some(-1),
    };
    let formatted = format_set(&set);
    assert_eq!(formatted, "BW-10 x 5");
}
