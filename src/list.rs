use crate::api::ApiClient;
use crate::workouts;
use crate::utils;
use crate::formatters;
use crate::table::{parse_date_and_filter_arguments, matches_any_filter};

pub async fn handle_list<C: ApiClient>(
    list: &crate::ListArgs,
    data_access: crate::api::DataAccess<'_, C>,
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

    if filters.is_empty() && !list.details && !list.summary {
        for date in dates_to_use {
            println!("{}", date);
        }
        return;
    }

    let workouts = match workouts::get_jdays(&data_access, &dates_to_use, verbose).await {
        Ok(w) => w,
        Err(e) => utils::exit_with_error(e),
    };

    let workouts: Vec<_> = if filters.is_empty() {
        workouts
    } else {
        workouts.into_iter().filter(|(_, jday)| {
            jday.eblocks.iter().any(|eblock| {
                jday.exercises.iter().any(|ex_wrap| {
                    ex_wrap.exercise.id == eblock.eid
                        && matches_any_filter(&ex_wrap.exercise.name, &filters)
                })
            })
        }).collect()
    };

    if workouts.is_empty() {
        utils::exit_with_error("No workouts found matching the specified filters");
    }

    if list.details || list.summary {
        let user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
        for (date, jday) in workouts {
            if list.details {
                let workout = formatters::format_workout(&date, &jday, user_wants_kg);
                println!("{}", workout);
                if !workout.ends_with('\n') {
                    println!();
                }
            } else {
                let fmt_date = formatters::color_date(&date);
                let summary = formatters::summarize_workout(&jday, user_wants_kg, &filters);
                println!("{} {}", fmt_date, summary);
            }
        }
    } else {
        for (date, _) in workouts {
            println!("{}", date);
        }
    }
}