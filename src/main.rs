mod models;
mod formatters;
mod auth;
mod api;
mod workouts;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wxrust")]
#[command(about = "WeightXReps Rust client")]
struct Args {
    #[arg(short, long, default_value = "credentials.txt")]
    credentials: String,

    #[arg(short = 'a', long = "force-authentication")]
    force_auth: bool,

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

    dates: Vec<String>,
}

#[derive(Parser)]
struct ShowArgs {
    #[arg(short, long)]
    summary: bool,

    date: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let token_path = format!("{}/.config/wxrust/token", home);

    match args.command {
        Commands::List(list) => {
            // Placeholder for list command
            println!("List command: details={}, summary={}, dates={:?}", list.details, list.summary, list.dates);
        }
        Commands::Show(show) => {
            let token = match auth::login(&args.credentials, &token_path).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            let workout = match workouts::get_day(&token, &show.date).await {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            println!("{}", workout);
        }
    }

    Ok(())
}
