mod models;
mod formatters;
mod auth;
mod api;
mod workouts;
mod utils;

use clap::{Parser, Subcommand};

use crate::api::{ReqwestClient, ApiClient};

#[derive(Parser)]
#[command(name = "wxrust")]
#[command(about = "WeightXReps Rust client")]
struct Args {
    #[arg(short, long, default_value = "credentials.txt")]
    credentials: String,

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

    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let token_path = format!("{}/.config/wxrust/token", home);

    match args.command {
        Commands::List(list) => {
            let client = ReqwestClient::new_with_verbose(args.verbose);
            let token = match auth::login(&client, &args.credentials, &token_path).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let user = match client.get_user_info(&token).await {
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
                for date in &dates_to_use {
                    let date = date.clone();
                    let client_clone = client.clone();
                    let token_clone = token.clone();
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let result = match workouts::get_jday(&client_clone, &token_clone, &date).await {
                            Ok(j) => Some(j),
                            Err(e) => {
                                eprintln!("Error getting workout for {}: {}", date, e);
                                None
                            }
                        };
                        tx_clone.send((date, result)).await.unwrap();
                    });
                }
                drop(tx);
                if list.details {
                    while let Some((date, result)) = rx.recv().await {
                        if let Some(jday) = result {
                            let text = formatters::render_workout(&date, &jday, &user);
                            println!("{}", text);
                        }
                    }
                } else if list.summary {
                    while let Some((date, result)) = rx.recv().await {
                        if let Some(j) = result {
                            let summary = formatters::summarize_workout(&j);
                            println!("{} {}", formatters::color_date(&date), summary);
                        }
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
            let token = match auth::login(&client, &args.credentials, &token_path).await {
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

            if show.summary {
                let jday = match workouts::get_jday(&client, &token, &date).await {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                let summary = formatters::summarize_workout(&jday);
                println!("{} {}", formatters::color_date(&date), summary);
            } else {
                let workout = match workouts::get_day(&client, &token, &date).await {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                println!("{}", workout);
            }
        }
    }

    Ok(())
}
