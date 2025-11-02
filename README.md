# WeightXReps Workout Extractor

[![codecov](https://codecov.io/gh/bartman/wxr-rs/branch/master/graph/badge.svg)](https://app.codecov.io/github/bartman/wxr-rs)

A Rust CLI tool to extract and display workouts from the WeightXReps.net website. It authenticates using credentials from a file and retrieves workout data via the GraphQL API, with support for listing, showing, and summarizing workouts.

## Features

- Authenticates using email and password from `credentials.txt` (email first line, password second)
- Retrieves individual workouts or lists of workout dates
- Formats output with colors matching the website editor
- Supports detailed views, summaries, and date filtering
- Handles JWT tokens and GraphQL queries efficiently

## Installation

Clone the repository and build with Cargo:

```bash
git clone <repo-url>
cd wxr-rs
cargo build --release
```

## Setup

Create `credentials.txt` in the project root with your WeightXReps email on the first line and password on the second.

## Usage

### Global Options

- `--credentials <file>`: Path to credentials file (default: `credentials.txt`)
- `--force-authentication`: Force re-login, ignore cached token
- `--color <always|never|auto>`: Control color output (default: auto, based on TTY)

### Commands

#### Show Workouts

- Show the most recent workout: `wxrust show`
- Show workout for a specific date: `wxrust show 2025-10-31`
- Show summary of recent workout: `wxrust show --summary`

#### List Workouts

- List recent workout dates: `wxrust list --count 5`
- List with full details: `wxrust list --details --count 3`
- List with summaries: `wxrust list --summary --count 2`
- List before a date: `wxrust list --before 2025-10-30 --count 5`
- Reverse order: `wxrust list --count 5 --reverse`
- List all (up to 1000): `wxrust list --all`

### Examples

```bash
# Show last workout
wxrust show

# List 5 recent workout dates
wxrust list --count 5

# Show detailed view of last 2 workouts
wxrust list --details --count 2

# Summarize workouts before a specific date
wxrust list --summary --before 2025-10-30 --count 3

# Disable colors
wxrust --color never list --summary --count 1
```

## Dependencies

- `reqwest`: HTTP client with connection reuse
- `serde`: JSON serialization
- `base64`: JWT decoding
- `tokio`: Async runtime
- `ansi_term`: Terminal colors
- `atty`: TTY detection
- `lazy_static`: Global color state
- `clap`: CLI parsing
- `regex`: Text processing
- `chrono`: Date handling

## API Details

Interacts with WeightXReps GraphQL API at `https://weightxreps.net/api/graphql`. Uses `login` mutation for auth, `jrange` query for date ranges, and `JDay` query for individual workouts. Supports efficient connection reuse for multiple requests.
