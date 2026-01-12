use crate::api::ReqwestClient;
use crate::formatters;
use crate::models;
use crate::parsers;
use crate::utils;
use crate::workouts;
use regex::Regex;
use std::fs;

pub async fn fetch_command(
    data_access: &crate::api::DataAccess<'_, ReqwestClient>,
    dates: &[String],
    diff: bool,
    force: bool,
    file: Option<&str>,
    verbose: bool,
) -> Result<(), String> {
    let uid = data_access.uid.ok_or("No user ID available")?;

    if let Some(file_path) = file {
        return fetch_from_file(uid, file_path, verbose);
    }

    let dates_to_fetch = get_dates_to_fetch(data_access, dates).await?;

    if dates_to_fetch.is_empty() {
        println!("No dates to fetch");
        return Ok(());
    }

    let pb = utils::create_progress_bar(dates_to_fetch.len() as u64);

    for date in &dates_to_fetch {
        pb.set_message(format!("Fetching {}", date));

        if diff {
            let server_jday = workouts::get_jday(data_access, date, verbose).await?;
            let local_jday = workouts::lookup_cached_jday(uid, date, verbose);

            if let Some(local) = local_jday {
                show_diff(date, &local, &server_jday);
            } else {
                println!("{}: No local cache, server version:", date);
                let user_wants_kg = workouts::resolve_user_wants_kg(data_access).await;
                let workout = formatters::format_workout(date, &server_jday, user_wants_kg);
                print!("{}", workout);
            }
        } else {
            if !force && workouts::lookup_cached_jday(uid, date, verbose).is_some() {
                pb.println(format!("{} already cached, skipping", date));
            } else {
                let jday = workouts::get_jday(data_access, date, verbose).await?;
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

    let pb = utils::create_progress_bar(workouts.len() as u64);

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
    data_access: &crate::api::DataAccess<'_, ReqwestClient>,
    dates: &[String],
) -> Result<Vec<String>, String> {
    if dates.is_empty() {
        workouts::get_dates(data_access, None, None, 10000, false).await
    } else {
        workouts::get_dates_from_ranges(data_access, dates).await
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
