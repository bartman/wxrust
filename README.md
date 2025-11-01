# WeightXReps Workout Extractor

A Rust tool to extract workouts from the WeightXReps.net website. This program authenticates with WeightXReps using credentials read from a text file and retrieves workout data via their GraphQL API.

## Features

- Authenticates using email and password from `credentials.txt` (on 2 separate lines)
- Retrieves workout data for a specified date
- Formats the output to match the site's workout log format
- Handles JWT token decoding and GraphQL queries

## Usage

1. Create a `credentials.txt` file with your email on the first line and password on the second.
2. Run the program: `cargo run`
3. The program will output the formatted workout for the date 2025-10-31.

## Dependencies

- reqwest for HTTP requests
- serde for JSON handling
- base64 for JWT decoding
- tokio for async runtime

## API Details

The tool interacts with WeightXReps' GraphQL API at `https://weightxreps.net/api/graphql`. It uses the `login` mutation for authentication and the `JDay` query for workout retrieval.
