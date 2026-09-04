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
mod list;

use clap::{Parser, Subcommand};

use wxrust::credentials;
use crate::api::{ReqwestClient, ApiClient};

#[derive(Parser)]
#[command(name = "wxrust")]
#[command(about = "WeightXReps Rust client")]
struct Args {
    /// Path to credentials.txt (email on line 1, password on line 2)
    #[arg(short, long, value_name = "FILE")]
    credentials: Option<String>,

    /// Ignore the cached auth token and log in again
    #[arg(short = 'a', long = "force-authentication")]
    force_auth: bool,

    /// Do not connect to the server; use local cache only
    #[arg(long)]
    no_network: bool,

    /// Do not read workouts from the local cache
    #[arg(long)]
    no_cache: bool,

    /// Do not write fetched workouts to the local cache
    #[arg(long)]
    no_cache_write: bool,

    /// When to color output: auto, always, never
    #[arg(long, default_value = "auto")]
    color: String,

    /// Enable debug output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List workout dates, optionally with details or summaries
    List(ListArgs),
    /// Show a workout log for a date (defaults to the most recent)
    Show(ShowArgs),
    /// Download workouts from the server into the local cache
    Fetch(FetchArgs),
    /// Display personal-record progression as a table
    Table(TableArgs),
    /// Display a calendar heatmap of workout intensity
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
    /// Print the full workout log for each date
    #[arg(short, long)]
    details: bool,

    /// Print a one-line summary of each workout
    #[arg(short, long)]
    summary: bool,

    /// List oldest first instead of newest first
    #[arg(short, long)]
    reverse: bool,

    /// List all workout dates (no count limit)
    #[arg(short = 'A', long)]
    all: bool,

    /// Only include workouts before this date (YYYY-MM-DD)
    #[arg(short, long, value_name = "DATE")]
    before: Option<String>,

    /// Maximum number of dates to list (default: 32)
    #[arg(short, long, value_name = "N")]
    count: Option<u32>,

    /// Date filters (YYYY, YYYY-MM, YYYY-MM-DD, 2025..2026) or exercise name substrings
    args: Vec<String>,
}

#[derive(Parser)]
struct ShowArgs {
    /// Print a one-line summary instead of the full workout log
    #[arg(short, long)]
    summary: bool,

    /// Workout date (YYYY-MM-DD). Defaults to the most recent workout
    date: Option<String>,
}

#[derive(Parser)]
struct FetchArgs {
    /// Compare local cache to the server instead of downloading
    #[arg(long)]
    diff: bool,

    /// Re-download workouts even if they are already cached
    #[arg(long)]
    force: bool,

    /// Import workouts from a text export file instead of the server
    #[arg(long, value_name = "FILE")]
    file: Option<String>,

    /// Print transfer rate (T/s, MB/s) after fetch
    #[arg(long)]
    stats: bool,

    /// Dates or ranges to fetch (YYYY, YYYY-MM, YYYY-MM-DD, 2025..). Defaults to all
    dates: Vec<String>,
}

#[derive(Parser)]
struct TableArgs {
    /// Hypothetical 1RM or set to compare against PRs (e.g. 500, 315x5); may be repeated
    #[arg(long, value_name = "SET")]
    dream: Vec<String>,

    /// Date filters (YYYY, YYYY-MM, YYYY-MM-DD, 2025..2026) or exercise name substrings
    args: Vec<String>,
}

#[derive(Parser)]
struct HeatmapArgs {
    /// Color days by number of sets
    #[arg(long, group = "metric")]
    sets: bool,

    /// Color days by total reps
    #[arg(long, group = "metric")]
    reps: bool,

    /// Color days by volume (weight x reps x sets)
    #[arg(long, group = "metric")]
    volume: bool,

    /// Color days by heaviest weight
    #[arg(long, group = "metric")]
    weight: bool,

    /// Color days by estimated 1RM (default)
    #[arg(long, group = "metric")]
    onerm: bool,

    /// Use a green RGB gradient instead of the default solarized colors
    #[arg(long)]
    green: bool,

    /// Date filters (YYYY, YYYY-MM, YYYY-MM-DD, 2025..2026) or exercise name substrings
    args: Vec<String>,
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
        let summary = formatters::summarize_workout(&jday, user_wants_kg, &[]);
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
        fetch_args.stats,
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
            list::handle_list(&list, &client, &token, data_access, args.verbose).await;
        },
        Commands::Show(show) => {
            handle_show(&show, data_access, args.verbose).await;
        },
        Commands::Fetch(fetch_args) => {
            handle_fetch(&fetch_args, data_access, args.verbose).await;
        },
        Commands::Table(table_args) => {
            table::handle_table(&client, &token, data_access, &table_args.args, &table_args.dream, args.verbose).await;
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
