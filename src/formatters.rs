use std::collections::HashMap;
use regex::Regex;
use lazy_static::lazy_static;
use ansi_term::Colour;
use atty;

use crate::models::{JDay, Set, Exercise};

lazy_static! {
    static ref COLOR_ENABLED: bool = {
        let color_arg = std::env::var("WXRUST_COLOR").unwrap_or("auto".to_string());
        match color_arg.as_str() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(atty::Stream::Stdout),
            _ => atty::is(atty::Stream::Stdout),
        }
    };

    pub static ref STDERR_COLOR_ENABLED: bool = {
        let color_arg = std::env::var("WXRUST_COLOR").unwrap_or("auto".to_string());
        match color_arg.as_str() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(atty::Stream::Stderr),
            _ => atty::is(atty::Stream::Stderr),
        }
    };
}

pub fn color_date(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(157, 78, 221).paint(s).to_string()
    } else {
        s.to_string()
    }
}

pub fn color_bw(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(58, 134, 255).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_exercise(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(0, 150, 255).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_weight(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(255, 121, 0).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_reps(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(0, 187, 249).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_sets(s: &str) -> String {
    if *COLOR_ENABLED {
        Colour::RGB(241, 91, 181).paint(s).to_string()
    } else {
        s.to_string()
    }
}



pub fn format_weight(w: f32, lb: bool) -> String {
    if lb {
        format!("{:.0}", w * 2.20462)
    } else {
        format!("{:.0}", w)
    }
}

pub fn format_set(set: &Set) -> String {
    let w = set.w.unwrap_or(0.0);
    let r = set.r.unwrap_or(0);
    let s = set.s.unwrap_or(1);
    let rpe = set.rpe.unwrap_or(0.0);
    let lb = set.lb.unwrap_or(0.0) == 1.0;
    let w_str = color_weight(&format_weight(w, lb));
    let mut line = w_str;
    if r > 0 {
        line += " x ";
        line += &color_reps(&r.to_string());
    }
    if s > 1 {
        line += " x ";
        line += &color_sets(&s.to_string());
    }
    if rpe > 0.0 {
        line += &format!(" @{}", rpe);
    }
    if let Some(c) = &set.c {
        if !c.is_empty() {
            line += &format!(" {}", c);
        }
    }
    line
}

pub fn compress_sets(sets: &[Set]) -> Vec<String> {
    let mut compressed = Vec::new();
    let mut i = 0;
    while i < sets.len() {
        let set = &sets[i];
        if set.set_type.unwrap_or(0) != 0 {
            compressed.push(format_set(set));
            i += 1;
            continue;
        }
        let w = set.w.unwrap_or(0.0);
        let r = set.r.unwrap_or(0);
        let _s = set.s.unwrap_or(1);
        let rpe = set.rpe.unwrap_or(0.0);
        let lb = set.lb.unwrap_or(0.0) == 1.0;
        // check for same weight consecutive
        let mut same_weight = vec![r];
        let mut j = i + 1;
        while j < sets.len() {
            let next = &sets[j];
            if next.set_type.unwrap_or(0) != 0 || next.w != set.w || next.rpe != set.rpe || next.lb != set.lb || next.s != set.s {
                break;
            }
            same_weight.push(next.r.unwrap_or(0));
            j += 1;
        }
        if same_weight.len() > 1 {
            let w_str = color_weight(&format_weight(w, lb));
            let r_str = same_weight.iter().map(|&r| color_reps(&r.to_string())).collect::<Vec<_>>().join(", ");
            let mut line = format!("{} x {}", w_str, r_str);
            if rpe > 0.0 {
                line += &format!(" @{}", rpe);
            }
            compressed.push(line);
            i = j;
        } else {
            // check for same rep
            let mut same_rep = vec![w];
            let mut j = i + 1;
            while j < sets.len() {
                let next = &sets[j];
                if next.set_type.unwrap_or(0) != 0 || next.r != set.r || next.rpe != set.rpe || next.lb != set.lb || next.s != set.s {
                    break;
                }
                same_rep.push(next.w.unwrap_or(0.0));
                j += 1;
            }
            if same_rep.len() > 1 {
                let w_str = same_rep.iter().map(|&w| color_weight(&format_weight(w, lb))).collect::<Vec<_>>().join(", ");
                let r_str = color_reps(&r.to_string());
                let mut line = format!("{} x {}", w_str, r_str);
                if rpe > 0.0 {
                    line += &format!(" @{}", rpe);
                }
                compressed.push(line);
                i = j;
            } else {
                compressed.push(format_set(set));
                i += 1;
            }
        }
    }
    compressed
}

pub fn format_eblocks(jday: &JDay) -> String {
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }
    let mut lines = Vec::new();
    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            lines.push("#".to_string() + &color_exercise(&ex.name));
            lines.extend(compress_sets(&eblock.sets));
        }
    }
    lines.join("\n")
}

pub fn summarize_workout(jday: &JDay) -> String {
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }
    let mut summaries = Vec::new();
    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            // Find the heaviest set: max weight, then max reps
            let mut max_weight = 0.0;
            let mut max_reps = 0;
            for set in &eblock.sets {
                let w = set.w.unwrap_or(0.0);
                let r = set.r.unwrap_or(0);
                if w > max_weight || (w == max_weight && r > max_reps) {
                    max_weight = w;
                    max_reps = r;
                }
            }
            if max_weight > 0.0 {
                let lb = eblock.sets.iter().any(|s| s.lb.unwrap_or(0.0) == 1.0);
                let w_str = color_weight(&format_weight(max_weight, lb));
                let r_str = color_reps(&max_reps.to_string());
                summaries.push(format!("#{}  {}x{}", color_exercise(&ex.name), w_str, r_str));
            }
        }
    }
    summaries.join("; ")
}

pub fn format_workout(jday: &JDay) -> String {
    let formatted_eblocks = format_eblocks(jday);
    let re = Regex::new(r"EBLOCK:\d+").unwrap();
    re.replace_all(&jday.log, &("\n".to_string() + &formatted_eblocks + "\n")).to_string()
}