use std::collections::HashMap;
use std::ops::Index;

use chrono::{NaiveDate, Utc, Datelike};
use lazy_static::lazy_static;
use ansi_term::Colour;

use crate::api::{ApiClient, DataAccess};
use crate::models::{JDay, EBlock, Exercise, ExerciseWrapper};
use crate::workouts;
use crate::utils;
use crate::formatters::STDERR_COLOR_ENABLED;
use crate::parsers::{LBS_PER_KG, ParserOptions, parse_set_line_with_options};

/// Maximum reps to track for rep-specific PRs
const MAX_REPS: usize = 10;

const ONERM_FACTOR: f32 = 36.0;

/// Color gradient for age-based coloring (256-color ANSI codes)
/// From oldest (cool colors) to newest (warm colors)
pub const GRADIENT: [u8; 20] = [
    0x23, 0x24, 0x25, 0x20, 0x21, 0x3f, 0x39, 0x5d,
    0x81, 0xa5, 0xc9, 0xc8, 0xc7, 0xc6, 0xc5, 0xc4,
    0xca, 0xd0, 0xd6, 0xdc,
];

/// Bright yellow for recent records (within 7 days)
pub const BRIGHT_YELLOW: u8 = 11;
/// Bright black (gray) for dimmed text
const BRIGHT_BLACK: u8 = 8;
/// Dark purple background for dream entries
const DREAM_BG: u8 = 53;

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
}

// ============================================================================
// 1RM Calculation Functions (Brzycki formula)
// ============================================================================

/// Calculate estimated 1RM from weight and reps using Brzycki formula
/// Formula: 1RM = weight * (36.0 / (37.0 - reps))
pub fn calculate_1rm(weight: f32, reps: u32) -> f32 {
    if weight <= 0.0 || reps == 0 {
        return 0.0;
    }
    if reps == 1 {
        return weight;
    }
    if reps > ONERM_FACTOR as u32 {
        // Brzycki formula breaks down at 37+ reps
        return weight * 2.0; // Rough estimate
    }
    weight * (ONERM_FACTOR / (ONERM_FACTOR + 1.0 - (reps as f32)))
}

/// Calculate weight for a given 1RM and target reps using Brzycki formula
/// Formula: weight = 1RM / (36.0 / (37.0 - reps))
pub fn weight_from_1rm(onerm: f32, reps: u32) -> f32 {
    if onerm <= 0.0 || reps == 0 {
        return 0.0;
    }
    if reps == 1 {
        return onerm;
    }
    if reps > ONERM_FACTOR as u32 {
        return onerm / 2.0; // Rough estimate
    }
    onerm / (ONERM_FACTOR / (ONERM_FACTOR + 1.0 - (reps as f32)))
}

/// Convert weight from kg to display units
/// Weights in the API are stored in kg; convert to lbs if user_wants_kg is false
pub fn convert_weight_for_display(weight_kg: f32, user_wants_kg: bool) -> f32 {
    if user_wants_kg {
        weight_kg
    } else {
        weight_kg * LBS_PER_KG
    }
}

// ============================================================================
// Argument Classification
// ============================================================================

/// Check if a string looks like a date argument
/// Supports: YYYY, YYYY-MM, YYYY.MM, YYYYMM, YYYY-MM-DD, YYYY.MM.DD, YYYYMMDD
pub fn is_date_arg(s: &str) -> bool {
    // Try to parse as a date boundary - if it succeeds, it's a date
    utils::parse_date_boundary(s, false).is_ok()
}

/// Parse arguments into (dates, exercise_filters)
/// Dates are arguments that look like dates, everything else is an exercise filter
pub fn parse_date_and_filter_arguments(args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut dates = Vec::new();
    let mut filters = Vec::new();

    for arg in args {
        if is_date_arg(arg) {
            dates.push(arg.clone());
        } else {
            filters.push(arg.clone());
        }
    }

    (dates, filters)
}

// ============================================================================
// Record Structure and State
// ============================================================================

/// A record of a personal record (PR) set
#[derive(Debug, Clone)]
pub struct Record {
    pub date: String,
    pub exercise_name: String,
    pub body_weight: Option<f32>,
    pub best_weight: f32,
    pub best_reps: u32,
    pub best_1rm: f32,
    /// True if this record came from a --dream injection
    pub is_dream: bool,
}

/// State for tracking PRs during processing
pub struct TableState {
    /// Best 1RM seen for each rep count (1-10)
    pub best_1rm_for_reps: [f32; MAX_REPS + 1],
    pub best_1rm_index: [isize; MAX_REPS + 1],
    /// All records collected
    pub records: Vec<Record>,
    /// Maximum exercise name width for formatting
    pub max_lift_width: usize,
}

impl TableState {
    pub fn new() -> Self {
        TableState {
            best_1rm_for_reps: [0.0; MAX_REPS + 1],
            best_1rm_index: [-1; MAX_REPS + 1],
            records: Vec::new(),
            max_lift_width: 0,
        }
    }

    /// Process a single set, potentially adding a record
    pub fn process_set(
        &mut self,
        date: &str,
        exercise_name: &str,
        body_weight: Option<f32>,
        weight: f32,
        reps: u32,
        is_dream: bool,
    ) {
        if weight <= 0.0 || reps == 0 {
            return;
        }

        let ent_1rm = calculate_1rm(weight, reps);
        let r = if reps > MAX_REPS as u32 { MAX_REPS } else { reps as usize };

        // Check if this is a new PR for this rep range
        let is_new_rep_pr = r <= MAX_REPS && self.best_1rm_for_reps[r] < ent_1rm;
        if ! is_new_rep_pr && !is_dream { return; };

        // Check if this PR replaces an existing PR on the same day
        let old_index = self.best_1rm_index[r];
        let mut replacing_old_index =
        if old_index >= 0 {
            let old_index = old_index as usize;
            let old_record = self.records.index(old_index);

            old_record.date == date
        } else { false };

        if replacing_old_index {
            let old_index = old_index as usize;

            if self.records[old_index].best_reps == reps {

                self.records[old_index].date = date.to_string();
                self.records[old_index].best_weight = weight;
                self.records[old_index].best_1rm = ent_1rm;

            } else {
                replacing_old_index = false;
            }
        }

        if ! replacing_old_index {
            let record = Record {
                date: date.to_string(),
                exercise_name: exercise_name.to_string(),
                body_weight,
                best_weight: weight,
                best_reps: reps,
                best_1rm: ent_1rm,
                is_dream,
            };

            self.records.push(record);
            self.best_1rm_for_reps[r] = ent_1rm;
            self.best_1rm_index[r] = (self.records.len() - 1) as isize;
        };

        let width = exercise_name.len();
        if self.max_lift_width < width {
            self.max_lift_width = width;
        }
    }
}

impl Default for TableState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Exercise Filtering
// ============================================================================

/// Check if an exercise name matches any of the filters (case-insensitive substring match)
pub fn matches_any_filter(exercise_name: &str, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true; // No filters = match all
    }
    let name_lower = exercise_name.to_lowercase();
    filters.iter().any(|f| name_lower.contains(&f.to_lowercase()))
}

// ============================================================================
// Workout Processing
// ============================================================================

/// Process a single workout, extracting records for matching exercises
pub fn process_workout(
    jday: &JDay,
    date: &str,
    filters: &[String],
    state: &mut TableState,
    is_dream: bool,
) {
    // Build exercise ID -> Exercise map
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }

    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            // Check if this exercise matches our filters
            if !matches_any_filter(&ex.name, filters) {
                continue;
            }

            // Process each set in this exercise block
            for set in &eblock.sets {
                let weight = set.w.unwrap_or(0.0);
                let reps = set.r.unwrap_or(0);
                let sets = set.s.unwrap_or(0);

                if weight > 0.0 && reps > 0 && sets > 0 {
                    state.process_set(date, &ex.name, jday.bw, weight, reps, is_dream);
                }
            }
        }
    }
}

// ============================================================================
// Color Formatting
// ============================================================================

/// Get foreground color escape sequence for 256-color
fn fg_256(color: u8) -> String {
    if *COLOR_ENABLED {
        format!("\x1b[38;5;{}m", color)
    } else {
        String::new()
    }
}

/// Get background color escape sequence for 256-color
fn bg_256(color: u8) -> String {
    if *COLOR_ENABLED {
        format!("\x1b[48;5;{}m", color)
    } else {
        String::new()
    }
}

/// Reset color escape sequence
fn col_reset() -> &'static str {
    if *COLOR_ENABLED {
        "\x1b[0m"
    } else {
        ""
    }
}

/// Calculate color index based on days since start and total days
pub fn get_gradient_color(days_since_start: i64, total_days: i64, days_ago: i64) -> u8 {
    // Recent records (within 7 days) get bright yellow
    if days_ago <= 7 {
        return BRIGHT_YELLOW;
    }

    if total_days <= 0 {
        return GRADIENT[GRADIENT.len() - 1];
    }

    let g = (days_since_start * GRADIENT.len() as i64 / total_days) as usize;
    let g = g.min(GRADIENT.len() - 1);
    GRADIENT[g]
}

// ============================================================================
// Table Formatting
// ============================================================================

/// Format the table output
pub fn format_table(state: &TableState, filters: &[String], user_wants_kg: bool) -> String {
    if state.records.is_empty() {
        return "No records found.".to_string();
    }

    let mut output = String::new();

    // Sort records by 1RM (ascending, like C code)
    let mut sorted_records = state.records.clone();
    sorted_records.sort_by(|a, b| a.best_1rm.partial_cmp(&b.best_1rm).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate date range
    let dates: Vec<&str> = sorted_records.iter().map(|r| r.date.as_str()).collect();
    let oldest_date = dates.iter().min().unwrap_or(&"");
    let newest_date = dates.iter().max().unwrap_or(&"");

    let start_date = NaiveDate::parse_from_str(oldest_date, "%Y-%m-%d").ok();
    let end_date = NaiveDate::parse_from_str(newest_date, "%Y-%m-%d").ok();
    let today = Utc::now().date_naive();

    let total_days = match (start_date, end_date) {
        (Some(s), Some(e)) => (e - s).num_days(),
        _ => 0,
    };

    // Summary header
    let filter_str = if filters.is_empty() {
        "all exercises".to_string()
    } else {
        filters.join(", ")
    };
    output.push_str(&format!("There were {} records of {}, over {} days\n",
        sorted_records.len(), filter_str, total_days));

    // Track best weight lifted for each rep range
    let mut best_lifted: [f32; MAX_REPS + 1] = [0.0; MAX_REPS + 1];
    let mut best_col: [u8; MAX_REPS + 1] = [0; MAX_REPS + 1];

    // Ensure minimum width for formatting
    let lift_width = state.max_lift_width.max(4);

    // Print each record
    for record in &sorted_records {
        let record_date = NaiveDate::parse_from_str(&record.date, "%Y-%m-%d").ok();

        let days_ago = record_date.map(|d| (today - d).num_days()).unwrap_or(0);
        let days_since_start = match (record_date, start_date) {
            (Some(rd), Some(sd)) => (rd - sd).num_days(),
            _ => 0,
        };

        let col = get_gradient_color(days_since_start, total_days, days_ago);
        let full_reset = col_reset();
        let colbg  = if record.is_dream { bg_256(DREAM_BG) } else { bg_256(0) };
        let coltxt = format!("{}{}", colbg, fg_256(col));
        let coldim = format!("{}{}", colbg, fg_256(BRIGHT_BLACK));
        let reset  = format!("{}{}", colbg, fg_256(15));

        // Convert weights for display
        let display_weight = convert_weight_for_display(record.best_weight, user_wants_kg);
        let display_1rm = convert_weight_for_display(record.best_1rm, user_wants_kg);

        // Track best weight for this rep range (in display units)
        let r = if record.best_reps > MAX_REPS as u32 { MAX_REPS } else { record.best_reps as usize };
        if r <= MAX_REPS && best_lifted[r] < display_weight {
            best_lifted[r] = display_weight;
            best_col[r] = col;
        }

        // Format margin for high-rep sets (> MAX_REPS)
        let margin = if record.best_reps > MAX_REPS as u32 {
            format!(" {}{:.0}{} x {}",
                coltxt, display_weight, reset, record.best_reps)
        } else {
            String::new()
        };

        // Format body weight
        let bw_str = match record.body_weight {
            Some(bw) if bw > 0.0 => {
                let display_bw = convert_weight_for_display(bw, user_wants_kg);
                format!("{:5.1}", display_bw)
            }
            _ => "     ".to_string(),
        };

        // Main row
        output.push_str(&format!("{}{}{} | {}{:4}{} | {} | {:<lift_width$} | {}{:5.1}{} |",
            coltxt, record.date, reset,
            coltxt, days_ago, reset,
            bw_str,
            record.exercise_name,
            coltxt, display_1rm, reset,
            lift_width = lift_width,
        ));

        // Rep columns (1-10)
        for rep in 1..=MAX_REPS {
            let colrep = if rep == record.best_reps as usize {
                &coltxt
            } else {
                &coldim
            };

            // Calculate projected weight in kg, then convert for display
            let projected_weight_kg = weight_from_1rm(record.best_1rm, rep as u32);
            let projected_weight = convert_weight_for_display(projected_weight_kg, user_wants_kg);
            output.push_str(&format!("{}{:4.0}{} |", colrep, projected_weight, reset));
        }

        output.push_str(&margin);
        output.push('\n');
    }

    // Header row (printed at bottom like C code)
    output.push_str(&format!("{}{}{:<10} | {:>4} | {:>5} | {:<lift_width$} | {:>5} |",
        bg_256(17), fg_256(226),
        "date", "days", "BW", "lift", "1RM",
        lift_width = lift_width,
    ));
    for rep in 1..=MAX_REPS {
        output.push_str(&format!(" {:>3} |", rep));
    }
    output.push_str(&format!("{}\n", col_reset()));

    // Best weight lifted row
    output.push_str(&format!("{:>width$} |",
        "best weight lifted",
        width = 36 + lift_width,
    ));
    for rep in 1..=MAX_REPS {
        let mut best = true;
        for after in rep+1..MAX_REPS {
            if best_lifted[rep] <= best_lifted[after] {
                best = false;
                break;
            }
        }
        let colrep = if best { fg_256(best_col[rep]) } else { fg_256(BRIGHT_BLACK) };
        let reset = col_reset();
        let str = if best_lifted[rep] > 0.0 {
                &format!(" {}{:3.0}{} |", colrep, best_lifted[rep], reset)
            } else {
                "     |"
            };
        output.push_str(str);
    }
    output.push('\n');

    output
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Handle the table command
pub async fn handle_table<C: ApiClient + Clone + Send + Sync + 'static>(
    client: &C,
    token: &Option<String>,
    data_access: DataAccess<'_, C>,
    args: &[String],
    dream_opt: &Option<String>,
    verbose: bool,
) {
    // Parse arguments into dates and exercise filters
    let (date_args, filters) = parse_date_and_filter_arguments(args);

    if verbose {
        let msg = format!("Date args: {:?}, Filters: {:?}", date_args, filters);
        if *STDERR_COLOR_ENABLED {
            eprintln!("{}", Colour::Blue.paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    }

    // Get dates to process
    let dates = if date_args.is_empty() {
        // Default: get all dates from cache/server
        match workouts::get_dates(&data_access, None, None, 10000, false).await {
            Ok(d) => d,
            Err(e) => {
                utils::exit_with_error(format!("Failed to get dates: {}", e));
            }
        }
    } else {
        match workouts::get_dates_from_ranges(&data_access, &date_args).await {
            Ok(d) => d,
            Err(e) => {
                utils::exit_with_error(format!("Failed to get dates from ranges: {}", e));
            }
        }
    };

    if dates.is_empty() {
        utils::exit_with_error("No workouts found in the specified range");
    }

    if verbose {
        let msg = format!("Processing {} dates", dates.len());
        if *STDERR_COLOR_ENABLED {
            eprintln!("{}", Colour::Blue.paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    }

    // Fetch workouts asynchronously (like handle_list)
    let user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    for date in dates.iter() {
        let date = date.clone();
        let client_clone = client.clone();
        let token_clone = token.clone();
        let use_network = data_access.use_network;
        let use_cache = data_access.use_cache;
        let write_cache = data_access.write_cache;
        let uid = data_access.uid;
        let tx_clone = tx.clone();
        //let verbose = verbose;

        tokio::spawn(async move {
            let data_access_clone = crate::api::DataAccess {
                client: &client_clone,
                token: token_clone.as_deref(),
                uid,
                use_network,
                use_cache,
                write_cache,
            };
            let result = match workouts::get_jday(&data_access_clone, &date, verbose).await {
                Ok(jday) => Some(jday),
                Err(e) => {
                    if verbose {
                        eprintln!("Error getting workout for {}: {}", date, e);
                    }
                    None
                }
            };
            let _ = tx_clone.send((date.clone(), result)).await;
        });
    }
    drop(tx);

    // Collect results and process in date order for deterministic PR tracking
    let mut results = Vec::new();
    while let Some(result) = rx.recv().await {
        results.push(result);
    }

    if let Some(dream) = dream_opt {
        // Use today's date
        let today = Utc::now().date_naive();
        let date = format!("{:04}-{:02}-{:02}", today.year(), today.month(), today.day());

        if verbose {
            let msg = format!("Dream set: {}", dream);
            if *STDERR_COLOR_ENABLED {
                eprintln!("{}", Colour::Blue.paint(msg));
            } else {
                eprintln!("{}", msg);
            }
        }

        // Normalize: ensure spaces around 'x' so parse_set_line can tokenize correctly
        let dream_normalized = dream.replace("x", " x ");

        // Parse using user's preferred units (user_wants_kg tells parser how to interpret bare numbers)
        let options = ParserOptions::new(user_wants_kg);
        let sets = match parse_set_line_with_options(&dream_normalized, &options) {
            Ok(sets) => sets,
            Err(e) => {
                utils::exit_with_error(format!("Failed to parse dream '{}': {}", dream, e));
            }
        };

        // Use first filter as the exercise name, or "Dream" if no filters
        let exercise_name = filters.first().cloned().unwrap_or_else(|| "Dream".to_string());
        let exercise_id = exercise_name.clone();

        let jday = JDay {
            log: "dream".to_string(),
            bw: None,
            eblocks: vec![EBlock {
                eid: exercise_id.clone(),
                sets,
            }],
            exercises: vec![ExerciseWrapper {
                exercise: Exercise {
                    id: exercise_id,
                    name: exercise_name,
                    ex_type: None,
                },
            }],
        };

        results.push((date, Some(jday)));
    }

    // Sort by date to ensure chronological processing
    results.sort_by(|a, b| a.0.cmp(&b.0));

    let mut state = TableState::new();
    for (date, jday_opt) in results {
        if let Some(jday) = jday_opt {
            let is_dream = jday.log == "dream";
            process_workout(&jday, &date, &filters, &mut state, is_dream);
        }
    }

    // Format and print the table
    let output = format_table(&state, &filters, user_wants_kg);
    print!("{}", output);
}

