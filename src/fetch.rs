use crate::api::{ReqwestClient, ApiClient};
use crate::auth;
use crate::formatters;
use crate::models;
use crate::parsers;
use crate::utils;
use crate::workouts;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::fs;

pub async fn fetch_command(
    client: &ReqwestClient,
    token: &str,
    dates: &[String],
    diff: bool,
    force: bool,
    file: Option<&str>,
    verbose: bool,
) -> Result<(), String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    if let Some(file_path) = file {
        return fetch_from_file(uid, file_path, verbose);
    }

    let dates_to_fetch = get_dates_to_fetch(client, token, dates).await?;

    if dates_to_fetch.is_empty() {
        println!("No dates to fetch");
        return Ok(());
    }

    let pb = ProgressBar::new(dates_to_fetch.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for date in &dates_to_fetch {
        pb.set_message(format!("Fetching {}", date));

        if diff {
            let server_jday = workouts::get_jday(client, token, date, verbose).await?;
            let local_jday = workouts::lookup_cached_jday(uid, date, verbose);

            if let Some(local) = local_jday {
                show_diff(date, &local, &server_jday);
            } else {
                println!("{}: No local cache, server version:", date);
                let user_wants_kg = client.user_wants_kg(token).await;
                let workout = formatters::format_workout(date, &server_jday, user_wants_kg);
                print!("{}", workout);
            }
        } else {
            if !force && workouts::lookup_cached_jday(uid, date, verbose).is_some() {
                pb.println(format!("{} already cached, skipping", date));
            } else {
                let jday = workouts::get_jday(client, token, date, verbose).await?;
                workouts::write_cached_jday(uid, date, &jday);
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done");

    Ok(())
}

fn fetch_from_file(uid: u32, file_path: &str, _verbose: bool) -> Result<(), String> {
    let content = fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let workouts = parse_file_export(&content)?;

    let pb = ProgressBar::new(workouts.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for (date, jday) in workouts {
        pb.set_message(format!("Caching {}", date));
        workouts::write_cached_jday(uid, &date, &jday);
        pb.inc(1);
    }

    pb.finish_with_message("Done");

    Ok(())
}

fn parse_file_export(content: &str) -> Result<Vec<(String, models::JDay)>, String> {
    let lines: Vec<&str> = content.lines().collect();
    let date_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

    let mut workouts = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if date_regex.is_match(lines[i]) {
            let date = lines[i].to_string();
            i += 1;

            // Collect lines until next date or end
            let mut workout_lines = Vec::new();
            while i < lines.len() && !date_regex.is_match(lines[i]) {
                workout_lines.push(lines[i]);
                i += 1;
            }

            let workout_text = format!("{}\n{}", date, workout_lines.join("\n"));
            let jday = parsers::parse_workout(&workout_text).map_err(|e| format!("Failed to parse workout for {}: {}", date, e))?;
            workouts.push((date, jday));
        } else {
            i += 1;
        }
    }

    Ok(workouts)
}

async fn get_dates_to_fetch(
    client: &ReqwestClient,
    token: &str,
    dates: &[String],
) -> Result<Vec<String>, String> {
    if dates.is_empty() {
        // Fetch all dates
        workouts::get_dates(client, token, None, None, 10000, false).await
    } else {
        let mut all_dates: Vec<String> = vec![];
        for range_str in dates {
            let (oldest, latest) = match utils::parse_date_range(range_str) {
                Ok(start_end) => start_end,
                Err(e) => {
                    return Err(format!("Invalid date range '{}': {}", range_str, e));
                }
            };
            let count = ((oldest - latest).num_days().abs() + 1) as u32;
            let dates = match workouts::get_dates(client, token, Some(latest.to_string()), Some(oldest.to_string()), count, false).await {
                Ok(d) => d,
                Err(e) => {
                    return Err(format!("Error getting dates for range {}: {}", range_str, e));
                }
            };
            all_dates.extend(dates);
        }
        all_dates.sort();
        all_dates.dedup();
        Ok(all_dates)
    }
}

fn show_diff(date: &str, local: &models::JDay, server: &models::JDay) {
    let local_text = formatters::format_workout_for_cache(date, local);
    let server_text = formatters::format_workout_for_cache(date, server);

    println!("Diff for {}:", date);
    let diff = similar::TextDiff::from_lines(&local_text, &server_text);
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => print!("\x1b[31m-{}\x1b[0m", change),
            similar::ChangeTag::Insert => print!("\x1b[32m+{}\x1b[0m", change),
            similar::ChangeTag::Equal => print!(" {}", change),
        }
    }
    println!();
}