mod models;
mod formatters;
mod auth;
mod api;
mod workouts;
mod utils;
mod parsers;
mod fetch;
mod table;
mod heatmap;

use clap::{Parser, Subcommand};

use wxrust::credentials;
use crate::api::{ReqwestClient, ApiClient};

#[derive(Parser)]
#[command(name = "wxrust")]
#[command(about = "WeightXReps Rust client")]
struct Args {
    #[arg(short, long)]
    credentials: Option<String>,

    #[arg(short = 'a', long = "force-authentication")]
    force_auth: bool,

    #[arg(long)]
    no_network: bool,

    #[arg(long)]
    no_cache: bool,

    #[arg(long)]
    no_cache_write: bool,

    #[arg(long, default_value = "auto")]
    color: String,

    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List(ListArgs),
    Show(ShowArgs),
    Fetch(FetchArgs),
    Table(TableArgs),
    Heatmap(HeatmapArgs),
}

async fn setup_auth_and_data_access(
    client: &ReqwestClient,
    credentials_path: &str,
    token_path: &str,
    no_network: bool,
    force_auth: bool,
) -> (Option<String>, Option<u32>) {
    let (token, uid) = if no_network {
        // Load uid from cached token, no network login
        let uid = match auth::load_uid_from_cache(token_path) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Failed to load cached token: {}", e);
                eprintln!("Use without --no-network to authenticate first.");
                utils::exit_with_error("");
            }
        };
        (None, Some(uid))
    } else {
        // Normal login
        let token = match auth::login(client, credentials_path, token_path, force_auth).await {
            Ok(t) => t,
            Err(e) => utils::exit_with_error(e),
        };
        let uid = match auth::decode_token(&token) {
            Ok(claims) => claims.id,
            Err(e) => utils::exit_with_error(format!("Failed to decode token: {}", e)),
        };
        let _user = match client.get_user_info(&token).await {
            Ok(u) => u,
            Err(e) => utils::exit_with_error(e),
        };
        (Some(token), Some(uid))
    };

    (token, uid)
}

#[derive(Parser)]
struct ListArgs {
    #[arg(short, long)]
    details: bool,

    #[arg(short, long)]
    summary: bool,

    #[arg(short, long)]
    reverse: bool,

    #[arg(short = 'A', long)]
    all: bool,

    #[arg(short, long)]
    before: Option<String>,

    #[arg(short, long)]
    count: Option<u32>,

    dates: Vec<String>,
}

#[derive(Parser)]
struct ShowArgs {
    #[arg(short, long)]
    summary: bool,

    date: Option<String>,
}

#[derive(Parser)]
struct FetchArgs {
    #[arg(long)]
    diff: bool,

    #[arg(long)]
    force: bool,

    #[arg(long, value_name = "FILE")]
    file: Option<String>,

    dates: Vec<String>,
}

#[derive(Parser)]
struct TableArgs {
    /// Arguments can be dates (YYYY, YYYY-MM, YYYY-MM-DD, etc.) or exercise filters
    /// Dates filter the date range, non-dates filter by exercise name (substring match)
    args: Vec<String>,
}

#[derive(Parser)]
struct HeatmapArgs {
    #[arg(long, group = "metric")]
    sets: bool,

    #[arg(long, group = "metric")]
    reps: bool,

    #[arg(long, group = "metric")]
    volume: bool,

    #[arg(long, group = "metric")]
    weight: bool,

    #[arg(long, group = "metric")]
    onerm: bool,

    #[arg(long)]
    green: bool,

    /// Arguments can be dates (YYYY, YYYY-MM, YYYY-MM-DD, etc.) or exercise filters
    /// Dates filter the date range, non-dates filter by exercise name (substring match)
    args: Vec<String>,
}

async fn handle_list(
    list: &ListArgs,
    client: &ReqwestClient,
    token: &Option<String>,
    data_access: api::DataAccess<'_, ReqwestClient>,
    verbose: bool,
) {
    let dates_to_use = if list.dates.is_empty() {
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
        let mut all_dates = match workouts::get_dates_from_ranges(&data_access, &list.dates).await {
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

    if list.details || list.summary {
        let user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
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
                    let summary = formatters::summarize_workout(&jday, user_wants_kg);
                    println!("{} {}", fmt_date, summary);
                }
                next_seq += 1;
            }
        }
    } else {
        for date in dates_to_use {
            println!("{}", date);
        }
    }
}

async fn handle_show(
    show: &ShowArgs,
    data_access: api::DataAccess<'_, ReqwestClient>,
    verbose: bool,
) {
    let date = if let Some(d) = &show.date {
        d.clone()
    } else {
        // Show last workout
        let dates = match workouts::get_dates(&data_access, None, None, 1, false).await {
            Ok(d) => d,
            Err(e) => utils::exit_with_error(e),
        };
        if let Some(d) = dates.first() {
            d.clone()
        } else {
            utils::exit_with_error("No workouts found");
        }
    };

    let jday = match workouts::get_jday(&data_access, &date, verbose).await {
        Ok(j) => j,
        Err(e) => utils::exit_with_error(e),
    };

    let user_wants_kg = workouts::resolve_user_wants_kg(&data_access).await;
    if show.summary {
        let fmt_date = formatters::color_date(&date);
        let summary = formatters::summarize_workout(&jday, user_wants_kg);
        println!("{} {}", fmt_date, summary);
    } else {
        let workout = formatters::format_workout(&date, &jday, user_wants_kg);
        print!("{}", workout);
        if !workout.ends_with('\n') {
            println!();
        }
    }
}

async fn handle_fetch(
    fetch_args: &FetchArgs,
    data_access: api::DataAccess<'_, ReqwestClient>,
    verbose: bool,
) {
    if let Err(e) = fetch::fetch_command(
        &data_access,
        &fetch_args.dates,
        fetch_args.diff,
        fetch_args.force,
        fetch_args.file.as_deref(),
        verbose,
    ).await {
        utils::exit_with_error(e);
    }
}

#[cfg_attr(tarpaulin, ignore)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate mutually exclusive options
    if args.no_network && args.no_cache {
        utils::exit_with_error("Error: --no-network and --no-cache are mutually exclusive");
    }

    unsafe { std::env::set_var("WXRUST_COLOR", &args.color); }

    let token_path = match workouts::get_cache_base_dir() {
        Ok(dir) => dir.join("token").to_string_lossy().to_string(),
        Err(_) => {
            let home = std::env::var("HOME").unwrap_or(".".to_string());
            format!("{}/.cache/wxrust/token", home)
        }
    };

    // Set credentials path if provided
    if let Some(path) = &args.credentials {
        credentials::set_credentials_path(path);
    }

    // Ensure credentials path is available
    let credentials_path = match credentials::get_credentials_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            eprintln!();
            eprintln!("Please create it with email on first line and password on second line at one of these locations:");
            if let Some(config_dir) = dirs::config_dir() {
                eprintln!("- {}", config_dir.join("wxrust").join("credentials.txt").display());
            }
            if let Ok(home) = std::env::var("HOME") {
                eprintln!("- {}/.config/wxrust/credentials.txt", home);
            }
            eprintln!("- ./credentials.txt");
            utils::exit_with_error("");
        }
    };

    let client = ReqwestClient::new_with_verbose(args.verbose);
    let (token, uid) = setup_auth_and_data_access(&client, &credentials_path, &token_path, args.no_network, args.force_auth).await;

    let data_access = api::DataAccess {
        client: &client,
        token: token.as_deref(),
        uid,
        use_network: !args.no_network,
        use_cache: !args.no_cache,
        write_cache: !args.no_cache_write,
    };

    match args.command {
        Commands::List(list) => {
            handle_list(&list, &client, &token, data_access, args.verbose).await;
        },
        Commands::Show(show) => {
            handle_show(&show, data_access, args.verbose).await;
        },
        Commands::Fetch(fetch_args) => {
            handle_fetch(&fetch_args, data_access, args.verbose).await;
        },
        Commands::Table(table_args) => {
            table::handle_table(&client, &token, data_access, &table_args.args, args.verbose).await;
        },
        Commands::Heatmap(heatmap_args) => {
            // Determine metric - default to OneRm
            let metric = if heatmap_args.sets {
                heatmap::Metric::Sets
            } else if heatmap_args.reps {
                heatmap::Metric::Reps
            } else if heatmap_args.volume {
                heatmap::Metric::Volume
            } else if heatmap_args.weight {
                heatmap::Metric::Weight
            } else if heatmap_args.onerm {
                heatmap::Metric::OneRm
            } else {
                heatmap::Metric::OneRm // default
            };

            heatmap::handle_heatmap(
                &client,
                &token,
                data_access,
                metric,
                heatmap_args.green,
                &heatmap_args.args,
                args.verbose,
            ).await;
        }
    }

    Ok(())
}
