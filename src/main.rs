mod models;
mod formatters;
mod auth;
mod api;
mod workouts;
mod utils;
mod parsers;

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



#[cfg_attr(tarpaulin, ignore)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
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
            let token = match auth::login(&client, &credentials_path.as_str(), &token_path).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
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

                match workouts::get_dates(&client, &token, latest, oldest, count, list.reverse).await {
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
                    let dates = match workouts::get_dates(&client, &token,
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
                let (tx, mut rx) = tokio::sync::mpsc::channel(32);
                for (seq, date) in dates_to_use.iter().enumerate() {
                    let date = date.clone();
                    let client_clone = client.clone();
                    let token_clone = token.clone();
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let result = match workouts::get_day(&client_clone, &token_clone, &date, args.verbose).await {
                            Ok(text) => Some(text),
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
                let mut buffer: BTreeMap<usize, (String, Option<String>)> = BTreeMap::new();
                let mut next_seq = 0;
                while let Some((seq, date, result)) = rx.recv().await {
                    buffer.insert(seq, (date, result));
                    while let Some((d, r)) = buffer.remove(&next_seq) {
                        if list.details {
                            if let Some(text) = r {
                                println!("{}", text);
                            }
                        } else if list.summary {
                            // For summary, we still need JDay, so call get_jday separately
                            match workouts::get_jday(&client, &token, &d, args.verbose).await {
                                Ok(jday) => {
                                    let summary = formatters::summarize_workout(&jday);
                                    println!("{} {}", formatters::color_date(&d), summary);
                                }
                                Err(e) => {
                                    eprintln!("Error getting workout for {}: {}", d, e);
                                }
                            }
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
        Commands::Show(show) => {
            let client = ReqwestClient::new_with_verbose(args.verbose);
            let token = match auth::login(&client, &credentials_path.as_str(), &token_path).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            let date = if let Some(d) = show.date {
                d
            } else {
                // Show last workout
                let dates = match workouts::get_dates(&client, &token, None, None, 1, false).await {
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

            let jday = match workouts::get_jday(&client, &token, &date, args.verbose).await {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let fmt_date = formatters::color_date(&date);

            if show.summary {
                let summary = formatters::summarize_workout(&jday);
                println!("{} {}", fmt_date, summary);
            } else {
                let workout = formatters::format_workout(&jday);
                println!("{}\n{}", fmt_date, workout);
            }
        }
    }

    Ok(())
}
