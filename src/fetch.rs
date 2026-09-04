use crate::api::ApiClient;
use crate::formatters;
use crate::models;
use crate::parsers;
use crate::utils;
use crate::workouts;
use regex::Regex;
use std::fs;
use std::time::Instant;

pub fn format_transfer_stats(transfers: u64, bytes: u64, elapsed_secs: f64) -> String {
    let (tps, mbps) = if elapsed_secs > 0.0 {
        (
            transfers as f64 / elapsed_secs,
            (bytes as f64 / 1_000_000.0) / elapsed_secs,
        )
    } else {
        (0.0, 0.0)
    };
    format!(
        "{} workouts, {:.2} seconds, {:.1} T/s, {:.2} MB/s",
        transfers,
        elapsed_secs.max(0.0),
        tps,
        mbps
    )
}

pub async fn fetch_command<C: ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    dates: &[String],
    diff: bool,
    force: bool,
    file: Option<&str>,
    verbose: bool,
    stats: bool,
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

    if diff {
        return fetch_diff(data_access, uid, &dates_to_fetch, verbose, stats).await;
    }

    let pb = utils::create_progress_bar(dates_to_fetch.len() as u64);

    let mut need = Vec::new();
    for date in &dates_to_fetch {
        if !force && workouts::cached_jday_exists(uid, date) {
            pb.println(format!("{} already cached, skipping", date));
            pb.inc(1);
        } else {
            need.push(date.clone());
        }
    }

    let mut elapsed_secs = 0.0;
    let mut bytes = 0u64;
    if !need.is_empty() {
        crate::api::reset_transfer_stats();
        let start = Instant::now();
        // --force must hit the network even when cache files already exist.
        let fetch_access = without_cache(data_access, force);
        workouts::get_jdays_with_callback(
            &fetch_access,
            &need,
            workouts::JDAY_BATCH_SIZE,
            workouts::FETCH_CONCURRENCY,
            verbose,
            |date, jday| {
                workouts::write_cached_jday(uid, date, jday);
                pb.set_message(format!("Fetching {}", date));
                pb.inc(1);
            },
        )
        .await?;
        elapsed_secs = start.elapsed().as_secs_f64();
        let (_requests, received) = crate::api::transfer_stats();
        bytes = received;
        if verbose {
            eprintln!(
                "Fetched {} workouts in {:.2}s ({} per request, concurrency {})",
                need.len(),
                elapsed_secs,
                workouts::JDAY_BATCH_SIZE,
                workouts::FETCH_CONCURRENCY
            );
        }
    }

    pb.finish_with_message("Done");

    if stats {
        println!(
            "{}",
            format_transfer_stats(need.len() as u64, bytes, elapsed_secs)
        );
    }

    Ok(())
}

async fn fetch_diff<C: ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    uid: u32,
    dates: &[String],
    verbose: bool,
    stats: bool,
) -> Result<(), String> {
    let pb = utils::create_progress_bar(dates.len() as u64);
    pb.set_message("Fetching");
    let user_wants_kg = workouts::resolve_user_wants_kg(data_access).await;

    crate::api::reset_transfer_stats();
    let start = Instant::now();
    // Diff compares against the server, so skip local cache for the download.
    let fetch_access = without_cache(data_access, true);
    let fetched = workouts::get_jdays(&fetch_access, dates, verbose).await?;
    let elapsed_secs = start.elapsed().as_secs_f64();
    let (_requests, bytes) = crate::api::transfer_stats();
    pb.inc(dates.len() as u64);

    pb.finish_and_clear();

    if stats {
        println!(
            "{}",
            format_transfer_stats(fetched.len() as u64, bytes, elapsed_secs)
        );
    }

    for (date, server_jday) in fetched {
        let server_text = workouts::format_cached_jday_text(&date, &server_jday);
        if let Some(local_text) = workouts::read_cached_jday_text(uid, &date) {
            // Compare the cache file to what a fetch would write. Parsing the
            // cache and reformatting it is not a round-trip (parser adds a
            // trailing newline to `log`), so that path flagged every workout.
            show_diff(&date, &local_text, &server_text);
        } else {
            println!("{}: No local cache, server version:", date);
            let workout = formatters::format_workout(&date, &server_jday, user_wants_kg);
            print!("{}", workout);
        }
    }

    Ok(())
}

fn fetch_from_file(uid: u32, file_path: &str, _verbose: bool) -> Result<(), String> {
    let content =
        fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

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

    // Use cached user preference to parse the file
    let user_wants_kg = workouts::read_cached_user_wants_kg_or(true);
    let options = parsers::ParserOptions::new(user_wants_kg);

    let mut workouts_list = Vec::new();
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
            let jday = parsers::parse_workout_with_options(&workout_text, &options)
                .map_err(|e| format!("Failed to parse workout for {}: {}", date, e))?;
            workouts_list.push((date, jday));
        } else {
            i += 1;
        }
    }

    Ok(workouts_list)
}

fn without_cache<'a, C: ApiClient>(
    data_access: &crate::api::DataAccess<'a, C>,
    skip_cache: bool,
) -> crate::api::DataAccess<'a, C> {
    crate::api::DataAccess {
        client: data_access.client,
        token: data_access.token,
        uid: data_access.uid,
        use_network: data_access.use_network,
        use_cache: data_access.use_cache && !skip_cache,
        write_cache: data_access.write_cache,
        scan_days: data_access.scan_days,
    }
}

async fn get_dates_to_fetch<C: ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    dates: &[String],
) -> Result<Vec<String>, String> {
    if dates.is_empty() {
        workouts::get_dates(data_access, None, None, 10000, false).await
    } else {
        workouts::get_dates_from_ranges(data_access, dates).await
    }
}

fn show_diff(date: &str, local_text: &str, server_text: &str) {
    if let Some(diff) = format_text_diff(date, local_text, server_text) {
        print!("{}", diff);
        println!();
    }
}

/// Unified-style diff of local cache text vs server text. `None` if identical.
pub fn format_text_diff(date: &str, local_text: &str, server_text: &str) -> Option<String> {
    if local_text == server_text {
        return None;
    }
    let mut out = format!("Diff for {}:\n", date);
    let diff = similar::TextDiff::from_lines(local_text, server_text);
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => out.push_str(&format!("\x1b[31m-{}\x1b[0m", change)),
            similar::ChangeTag::Insert => out.push_str(&format!("\x1b[32m+{}\x1b[0m", change)),
            similar::ChangeTag::Equal => out.push_str(&format!(" {}", change)),
        }
    }
    Some(out)
}
