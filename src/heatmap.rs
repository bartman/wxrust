use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Weekday, Month};
use num_traits::cast::FromPrimitive;

use crate::api::{ApiClient, DataAccess};
use crate::models::{JDay, Exercise};
use crate::workouts;
use crate::utils;
use crate::table::{calculate_1rm, parse_date_and_filter_arguments};

/// Metric to use for intensity calculation
#[derive(Debug, Clone, Copy)]
pub enum Metric {
    Sets,
    Reps,
    Volume,
    Weight,
    OneRm,
}

/// Compute the metric value for a single workout
pub fn compute_metric(jday: &JDay, metric: Metric, filters: &[String]) -> f64 {
    // Build exercise ID -> Exercise map
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }

    let mut total = 0.0f64;
    let mut max_val = 0.0f32;

    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            // Check if this exercise matches our filters
            if !filters.is_empty() {
                let name_lower = ex.name.to_lowercase();
                if !filters.iter().any(|f| name_lower.contains(&f.to_lowercase())) {
                    continue;
                }
            }

            // Process each set in this exercise block
            for set in &eblock.sets {
                let weight = set.w.unwrap_or(0.0);
                let reps = set.r.unwrap_or(0);
                let sets = set.s.unwrap_or(0);

                if weight > 0.0 && reps > 0 && sets > 0 {
                    match metric {
                        Metric::Sets => total += sets as f64,
                        Metric::Reps => total += reps as f64 * sets as f64,
                        Metric::Volume => total += weight as f64 * (reps as f64 * sets as f64),
                        Metric::Weight => {
                            if weight > max_val {
                                max_val = weight;
                            }
                        }
                        Metric::OneRm => {
                            let onerm = calculate_1rm(weight, reps);
                            if onerm > max_val {
                                max_val = onerm;
                            }
                        }
                    }
                }
            }
        }
    }

    match metric {
        Metric::Sets | Metric::Reps | Metric::Volume => total,
        Metric::Weight | Metric::OneRm => max_val as f64,
    }
}

/// Handle the heatmap command
pub async fn handle_heatmap<C: ApiClient + Clone + Send + Sync + 'static>(
    client: &C,
    token: &Option<String>,
    data_access: DataAccess<'_, C>,
    sets: bool,
    reps: bool,
    volume: bool,
    weight: bool,
    onerm: bool,
    green: bool,
    args: &[String],
    verbose: bool,
) {
    // Determine metric - default to OneRm
    let metric = if sets {
        Metric::Sets
    } else if reps {
        Metric::Reps
    } else if volume {
        Metric::Volume
    } else if weight {
        Metric::Weight
    } else if onerm {
        Metric::OneRm
    } else {
        Metric::OneRm // default
    };

    // Parse arguments into dates and exercise filters
    let (date_args, filters) = parse_date_and_filter_arguments(args);

    if verbose {
        let msg = format!("Date args: {:?}, Filters: {:?}", date_args, filters);
        if *crate::formatters::STDERR_COLOR_ENABLED {
            eprintln!("{}", ansi_term::Colour::Blue.paint(msg));
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
        if *crate::formatters::STDERR_COLOR_ENABLED {
            eprintln!("{}", ansi_term::Colour::Blue.paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    }

    // Fetch workouts asynchronously
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

    // Collect results
    let mut results = Vec::new();
    while let Some(result) = rx.recv().await {
        results.push(result);
    }

    // Sort by date
    results.sort_by(|a, b| a.0.cmp(&b.0));

    // Compute daily metrics
    let mut daily_values: HashMap<NaiveDate, f64> = HashMap::new();
    for (date_str, jday_opt) in results {
        if let Some(jday) = jday_opt {
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                let value = compute_metric(&jday, metric, &filters);
                if value > 0.0 {
                    daily_values.insert(date, value);
                }
            }
        }
    }

    if daily_values.is_empty() {
        println!("No data to display.");
        return;
    }

    // Find date range
    let mut date_list: Vec<NaiveDate> = daily_values.keys().cloned().collect();
    date_list.sort();
    let start_date = *date_list.first().unwrap();
    let end_date = *date_list.last().unwrap();

    // Find min and max values
    let max_value = daily_values.values().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_value = daily_values.values().cloned().fold(f64::INFINITY, f64::min);

    if verbose {
        let msg = format!("Range: {} .. {} kg", min_value, max_value);
        if *crate::formatters::STDERR_COLOR_ENABLED {
            eprintln!("{}", ansi_term::Colour::Blue.paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    }

    // Draw the heatmap
    draw_heatmap(daily_values, start_date, end_date, min_value, max_value, green);
}

/// Draw the heatmap to the console.
fn draw_heatmap(
    daily_values: HashMap<NaiveDate, f64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    min_value: f64,
    max_value: f64,
    green: bool,
) {
    let mut first_monday = start_date;
    while first_monday.weekday() != Weekday::Mon {
        first_monday = first_monday.pred_opt().unwrap();
    }

    let mut weeks: Vec<Vec<Option<f64>>> = Vec::new();
    let mut week_dates: Vec<NaiveDate> = Vec::new();
    let mut current_week = vec![None; 7];
    let mut current_date = first_monday;

    while current_date <= end_date {
        let day_of_week = current_date.weekday().num_days_from_monday() as usize;
        if day_of_week == 0 {
            week_dates.push(current_date);
        }
        current_week[day_of_week] = daily_values.get(&current_date).cloned();

        if current_date.weekday() == Weekday::Sun {
            weeks.push(current_week);
            current_week = vec![None; 7];
        }
        current_date = current_date.succ_opt().unwrap();
    }
    if !current_week.iter().all(Option::is_none) {
        weeks.push(current_week);
    }

    let terminal_width = if let Some((w, _)) = term_size::dimensions() {
        w
    } else {
        80 // Default width
    };
    let max_weeks = (terminal_width - 5) / 3;

    if weeks.len() > max_weeks {
        let start_index = weeks.len() - max_weeks;
        weeks = weeks.into_iter().skip(start_index).collect();
        week_dates = week_dates.into_iter().skip(start_index).collect();
    }

    // Print header
    print!("     ");
    for date in &week_dates {
        print!("{:2} ", date.day());
    }
    println!();

    // Print body
    for day_of_week in 0..7 {
        let day_label = match day_of_week {
            0 => "Mon ",
            2 => "Wed ",
            4 => "Fri ",
            6 => "Sun ",
            _ => "    ",
        };
        print!("{}", day_label);

        for (week_index, week) in weeks.iter().enumerate() {
            let cell = week[day_of_week];
            let current_day = week_dates[week_index] + chrono::Duration::days(day_of_week as i64);

            if current_day < start_date || current_day > end_date {
                print!("   ");
            } else {
                let symbol = match cell {
                    Some(value) if value > 0.0 => {
                        let range = max_value - min_value;
                        let intensity = if range > 0.0 {
                            ((value - min_value) / range * 230.0) as u8 + 25
                        } else {
                            128 // middle intensity if all values are the same
                        };
                        if *crate::formatters::COLOR_ENABLED {
                            let color_code = if green {
                                // Green gradient: RGB(0, intensity, 0)
                                format!("\x1b[38;2;0;{};0m", intensity)
                            } else {
                                // Default to solarized if neither specified
                                let gradient_index = (intensity as usize * (crate::table::GRADIENT.len() - 1) / 255).min(crate::table::GRADIENT.len() - 1);
                                format!("\x1b[38;5;{}m", crate::table::GRADIENT[gradient_index])
                            };
                            format!("{} ◀▶\x1b[0m", color_code)
                        } else {
                            // Map to 4 levels
                            let level = ((value / max_value * 3.0) as usize).min(3);
                            match level {
                                0 => " ◀▶",
                                1 => " ◁▷",
                                2 => " ◂▸",
                                3 => " ◃▹",
                                _ => " ◀▶",
                            }.to_string()
                        }
                    }
                    _ => {
                        if *crate::formatters::COLOR_ENABLED {
                            "\x1b[38;2;20;20;20m ◀▶\x1b[0m".to_string()
                        } else {
                            "   ".to_string()
                        }
                    }
                };
                print!("{}", symbol);
            }
        }
        println!();
    }

    // Print footer
    print!("     ");
    let mut last_month = 0;
    for date in &week_dates {
        if date.month() != last_month {
            print!("{:3}", Month::from_u32(date.month()).unwrap().name()[..3].to_string());
            last_month = date.month();
        } else {
            print!("   ");
        }
    }
    println!();
}
