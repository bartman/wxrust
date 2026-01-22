use wxrust::formatters::*;
use wxrust::models::{JDay, Set, Exercise, ExerciseWrapper, EBlock};

#[test]
fn test_format_weight() {
    let options = FormatOptions::no_color(true);
    assert_eq!(format_weight(100.0, false, &options), "100");
    assert_eq!(format_weight(100.0, true, &options), "45");  // 100 lbs to kg ≈ 45.35, rounded to 45
    assert_eq!(format_weight(45.5, false, &options), "46");   // Rounded
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
    let summary = summarize_workout(&jday, true);
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
    let formatted = format_workout("2023-10-01", &jday, true);
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
    let formatted = format_workout("2023-10-01", &jday, true);
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

#[test]
fn test_format_set_failed() {
    let set = Set {
        w: Some(247.208),
        r: Some(0),
        s: Some(1),
        lb: Some(1.0),
        rpe: Some(0.0),
        c: Some(":'(".to_string()),
        ..Default::default()
    };
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_set(&set);
    // With user_wants_kg = true, 247.208 lbs -> kg ≈ 112
    assert_eq!(formatted, "247 x 0 x 1 :'(");
}

#[test]
fn test_format_workout_with_failed_sets() {
    // JSON response from the user
    let json = r#"{"data":{"jday":{"log":"deadlift\nEBLOCK:54709\n--\nx#chinup\nBW0 x 5 x 3\nx#barbell-landmine-row-narrow-grip\n135, 160, 180 x 10\nx#barbell-cheat-shrugs\n315, 365, 405, 455 x 10\nx#rack-pulls #dl\n135 x 10\n225,315, 405, 495 x 5","bw":92.5329,"eblocks":[{"eid":"54709","sets":[{"w":61.235,"r":10,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":76.12996754574426,"eff":0.30241052722331635,"int":0.2432431358840727,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""},{"w":102.058,"r":5,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":111.77812555698762,"eff":0.44401545109567425,"int":0.40540390237701796,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""},{"w":142.882,"r":3,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":149.37622549324806,"eff":0.5933661153723603,"int":0.567568641159273,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":"R"},{"w":183.705,"r":2,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":187.78725717488848,"eff":0.745945982624461,"int":0.7297294076522182,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":"L"},{"w":224.528,"r":1,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":224.5282440185547,"eff":0.8918919426752364,"int":0.8918901741451634,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":"R"},{"w":256.28,"r":0,"s":2,"lb":1,"rpe":0,"pr":0,"est1rm":250.82693409970983,"eff":0.9963580417569943,"int":1.0180183043091393,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""},{"w":247.208,"r":0,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":241.94811356096886,"eff":0.9610887662430057,"int":0.9819816956908606,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":":'("},{"w":233.6,"r":1,"s":2,"lb":1,"rpe":0,"pr":0,"est1rm":233.60008239746094,"eff":0.9279279415793646,"int":0.9279267827634422,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""},{"w":210.92,"r":5,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":231.00813506417816,"eff":0.9176319676697196,"int":0.837835261217745,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""},{"w":170.097,"r":10,"s":1,"lb":1,"rpe":0,"pr":0,"est1rm":211.47213576019809,"eff":0.840029256939532,"int":0.6756744947247998,"type":0,"t":0,"d":0,"dunit":null,"speed":0,"force":null,"c":""}]}],"exercises":[{"exercise":{"id":"54709","name":"deadlift #dl","type":"DL"}}]}}} "#;
    let response: wxrust::models::WorkoutResponse = serde_json::from_str(json).unwrap();
    let jday = response.data.unwrap().jday.unwrap();
    unsafe { std::env::set_var("WXRUST_COLOR", "never"); }
    let formatted = format_workout("2023-02-01", &jday, false); // user_wants_kg = false, so show lbs
    // Check that the failed set is formatted as "247 x 0 x 1 :'("
    assert!(formatted.contains("545 x 0 x 1 :'("));
    // And not "247 x 1 :'("
    assert!(!formatted.contains("545 x 1 :'("));
}



