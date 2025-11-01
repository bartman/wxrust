mod models;
mod formatters;
mod auth;
mod api;
mod workouts;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let token_path = format!("{}/.config/wxrust/token", home);
    let token = match auth::login("credentials.txt", &token_path).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let workout = match workouts::get_day(&token, "2025-10-31").await {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("{}", workout);

    Ok(())
}
