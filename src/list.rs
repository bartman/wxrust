use crate::api::ReqwestClient;
use crate::models;
use crate::workouts;
use crate::utils;
use crate::formatters;
use crate::table::{parse_date_and_filter_arguments, matches_any_filter};

pub async fn handle_list(
    list: &crate::ListArgs,
    client: &ReqwestClient,
    token: &Option<String>,
    data_access: crate::api::DataAccess<'_, ReqwestClient>,
    verbose: bool,
) {
    let (date_args, filters) = parse_date_and_filter_arguments(&list.args);

    let dates_to_use = if date_args.is_empty() {
        let (latest, oldest, count) = if list.all {
            (None, None, 10000)
        } else if let Some(before) = &list.before {
            let cnt = list.count.unwrap_or(32);
            (Some(before.clone()), None, cnt)
        } else if let Some(cnt) = list.count {
            (None, None, cnt)
        } else {
            (None, None, 32)
        };

        match workouts::get_dates(&data_access, latest, oldest, count, list.reverse).await {
            Ok(d) => d,
            Err(e) => utils::exit_with_error(e),
        }
    } else {
        let mut all_dates = match workouts::get_dates_from_ranges(&data_access, &date_args).await {
            Ok(d) => d,
            Err(e) => utils::exit_with_error(e),
        };
        if list.reverse {
            all_dates.reverse();
        }
        all_dates
    };

    if dates_to_use.is_empty() {
        utils::exit_with_error("No workouts found in the specified range");
    }

    // If filters are present, we need to filter dates based on workout content
    let filtered_dates = if filters.is_empty() {
        dates_to_use
    } else {
        // Need to fetch workouts to check for matching exercises
        let _user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let filters_clone = filters.clone();
        for (seq, date) in dates_to_use.iter().enumerate() {
            let date = date.clone();
            let client_clone = client.clone();
            let token_clone = token.clone();
            let verbose = verbose;
            let use_network = data_access.use_network;
            let use_cache = data_access.use_cache;
            let write_cache = data_access.write_cache;
            let uid = data_access.uid;
            let tx_clone = tx.clone();
            let filters_clone = filters_clone.clone();
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
                    Ok(jday) => {
                        // Check if any exercise matches the filters
                        let mut has_match = false;
                        for eblock in &jday.eblocks {
                            if let Some(ex) = jday.exercises.iter().find(|ex_wrap| ex_wrap.exercise.id == eblock.eid) {
                                if matches_any_filter(&ex.exercise.name, &filters_clone) {
                                    has_match = true;
                                    break;
                                }
                            }
                        }
                        if has_match { Some(date.clone()) } else { None }
                    }
                    Err(e) => {
                        eprintln!("Error getting workout for {}: {}", date, e);
                        None
                    }
                };
                tx_clone.send((seq, result)).await.unwrap();
            });
        }
        drop(tx);
        use std::collections::BTreeMap;
        let mut buffer: BTreeMap<usize, Option<String>> = BTreeMap::new();
        let mut filtered = Vec::new();
        let mut next_seq = 0;
        while let Some((seq, result)) = rx.recv().await {
            buffer.insert(seq, result);
            while let Some(maybe_date) = buffer.remove(&next_seq) {
                if let Some(date) = maybe_date {
                    filtered.push(date);
                }
                next_seq += 1;
            }
        }
        filtered
    };

    if filtered_dates.is_empty() {
        utils::exit_with_error("No workouts found matching the specified filters");
    }

    if list.details || list.summary {
        let user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        for (seq, date) in filtered_dates.iter().enumerate() {
            let date = date.clone();
            let client_clone = client.clone();
            let token_clone = token.clone();
            let verbose = verbose;
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
                        eprintln!("Error getting workout for {}: {}", date, e);
                        None
                    }
                };
                tx_clone.send((seq, date.clone(), result)).await.unwrap();
            });
        }
        drop(tx);
        use std::collections::BTreeMap;
        let mut buffer: BTreeMap<usize, (String, Option<models::JDay>)> = BTreeMap::new();
        let mut next_seq = 0;
        while let Some((seq, new_date, new_jday)) = rx.recv().await {
            buffer.insert(seq, (new_date, new_jday));
            while let Some((date, result)) = buffer.remove(&next_seq) {
                let jday = match result {
                    Some(jday) => jday,
                    _ => continue
                };
                if list.details {
                    let workout = formatters::format_workout(&date, &jday, user_wants_kg);
                    println!("{}", workout);
                    if !workout.ends_with('\n') {
                        println!();
                    }
                } else if list.summary {
                    let fmt_date = formatters::color_date(&date);
                    let summary = formatters::summarize_workout(&jday, user_wants_kg, &filters);
                    println!("{} {}", fmt_date, summary);
                }
                next_seq += 1;
            }
        }
    } else {
        for date in filtered_dates {
            println!("{}", date);
        }
    }
}