mod models;
mod formatters;
mod auth;
mod api;
mod workouts;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = match auth::login("credentials.txt").await {
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

    println!("Full formatted workout:\n{}", workout);

    Ok(())
}