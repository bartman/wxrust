mod models;
mod formatters;
mod auth;
mod api;
mod workouts;
mod utils;
mod parsers;
mod fetch;

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



#[cfg_attr(tarpaulin, ignore)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate mutually exclusive options
    if args.no_network && args.no_cache {
        eprintln!("Error: --no-network and --no-cache are mutually exclusive");
        std::process::exit(1);
    }

    unsafe { std::env::set_var("WXRUST_COLOR", &args.color); }

    let token_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("wxrust").join("token").to_string_lossy().to_string()
    } else {
        // Fallback
        let home = std::env::var("HOME").unwrap_or(".".to_string());
        format!("{}/.config/wxrust/token", home)
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
            std::process::exit(1);
        }
    };

    match args.command {
        Commands::List(list) => {
            let client = ReqwestClient::new_with_verbose(args.verbose);

            let (token, uid) = if args.no_network {
                // Load uid from cached token, no network login
                let uid = match auth::load_uid_from_cache(&token_path) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("Failed to load cached token: {}", e);
                        eprintln!("Use without --no-network to authenticate first.");
                        std::process::exit(1);
                    }
                };
                (None, Some(uid))
            } else {
                // Normal login
                let token = match auth::login(&client, &credentials_path.as_str(), &token_path, args.force_auth).await {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                let uid = match auth::decode_token(&token) {
                    Ok(claims) => claims.id,
                    Err(e) => {
                        eprintln!("Failed to decode token: {}", e);
                        std::process::exit(1);
                    }
                };
                let _user = match client.get_user_info(&token).await {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                (Some(token), Some(uid))
            };

            let data_access = crate::api::DataAccess {
                client: &client,
                token: token.as_deref(),
                uid,
                use_network: !args.no_network,
                use_cache: !args.no_cache,
                write_cache: !args.no_cache_write,
            };
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
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Parse ranges
                let mut all_dates: Vec<String> = vec![];
                for range_str in &list.dates {
                    let (oldest, latest) = match utils::parse_date_range(range_str) {
                        Ok(start_end) => start_end,
                        Err(e) => {
                            eprintln!("Invalid date range '{}': {}", range_str, e);
                            std::process::exit(1);
                        }
                    };
                    let count = ((oldest - latest).num_days().abs() + 1) as u32;
                    let dates = match workouts::get_dates(&data_access,
                                        Some(latest.to_string()), Some(oldest.to_string()), count, false).await {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    };
                    all_dates.extend(dates);
                }
                all_dates.sort();
                if list.reverse {
                    all_dates.reverse();
                }
                all_dates
            };

            if dates_to_use.is_empty() {
                eprintln!("No workouts found in the specified range");
                std::process::exit(1);
            }

            if list.details || list.summary {
                let user_wants_kg = if let Some(token_ref) = &token {
                    data_access.client.user_wants_kg(token_ref).await
                } else {
                    // No network mode: use cached preference
                    crate::workouts::read_cached_user_wants_kg_or(true)
                };
                let (tx, mut rx) = tokio::sync::mpsc::channel(32);
                for (seq, date) in dates_to_use.iter().enumerate() {
                    let date = date.clone();
                    let client_clone = client.clone();
                    let data_access_clone = crate::api::DataAccess {
                        client: &client_clone,
                        token: token.as_deref(),
                        uid: data_access.uid,
                        use_network: data_access.use_network,
                        use_cache: data_access.use_cache,
                        write_cache: data_access.write_cache,
                    };
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let result = match workouts::get_jday(&data_access_clone, &date, args.verbose).await {
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
                            let summary = formatters::summarize_workout(&jday);
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
        },
        Commands::Show(show) => {
            let client = ReqwestClient::new_with_verbose(args.verbose);

            let (token, uid) = if args.no_network {
                // Load uid from cached token, no network login
                let uid = match auth::load_uid_from_cache(&token_path) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("Failed to load cached token: {}", e);
                        eprintln!("Use without --no-network to authenticate first.");
                        std::process::exit(1);
                    }
                };
                (None, Some(uid))
            } else {
                // Normal login
                let token = match auth::login(&client, &credentials_path.as_str(), &token_path, args.force_auth).await {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                let uid = match auth::decode_token(&token) {
                    Ok(claims) => claims.id,
                    Err(e) => {
                        eprintln!("Failed to decode token: {}", e);
                        std::process::exit(1);
                    }
                };
                (Some(token), Some(uid))
            };

            let data_access = crate::api::DataAccess {
                client: &client,
                token: token.as_deref(),
                uid,
                use_network: !args.no_network,
                use_cache: !args.no_cache,
                write_cache: !args.no_cache_write,
            };

            let date = if let Some(d) = show.date {
                d
            } else {
                // Show last workout
                let dates = match workouts::get_dates(&data_access, None, None, 1, false).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                if let Some(d) = dates.get(0) {
                    d.clone()
                } else {
                    eprintln!("No workouts found");
                    std::process::exit(1);
                }
            };

            let jday = match workouts::get_jday(&data_access, &date, args.verbose).await {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            if show.summary {
                let fmt_date = formatters::color_date(&date);
                let summary = formatters::summarize_workout(&jday);
                println!("{} {}", fmt_date, summary);
            } else {
                let user_wants_kg = if let Some(token_ref) = &token {
                    data_access.client.user_wants_kg(token_ref).await
                } else {
                    // No network mode: use cached preference
                    crate::workouts::read_cached_user_wants_kg_or(true)
                };
                let workout = formatters::format_workout(&date, &jday, user_wants_kg);
                print!("{}", workout);
                if !workout.ends_with('\n') {
                    println!();
                }
            }
        },
        Commands::Fetch(fetch_args) => {
            let client = ReqwestClient::new_with_verbose(args.verbose);

            let (token, uid) = if args.no_network {
                // Load uid from cached token, no network login
                let uid = match auth::load_uid_from_cache(&token_path) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("Failed to load cached token: {}", e);
                        eprintln!("Use without --no-network to authenticate first.");
                        std::process::exit(1);
                    }
                };
                (None, Some(uid))
            } else {
                // Normal login
                let token = match auth::login(&client, &credentials_path.as_str(), &token_path, args.force_auth).await {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                let uid = match auth::decode_token(&token) {
                    Ok(claims) => claims.id,
                    Err(e) => {
                        eprintln!("Failed to decode token: {}", e);
                        std::process::exit(1);
                    }
                };
                (Some(token), Some(uid))
            };

            let data_access = crate::api::DataAccess {
                client: &client,
                token: token.as_deref(),
                uid,
                use_network: !args.no_network,
                use_cache: !args.no_cache,
                write_cache: !args.no_cache_write,
            };

            if let Err(e) = fetch::fetch_command(
                &data_access,
                &fetch_args.dates,
                fetch_args.diff,
                fetch_args.force,
                fetch_args.file.as_deref(),
                args.verbose,
            ).await {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
