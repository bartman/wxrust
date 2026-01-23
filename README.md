# wxrust - WeightXReps Workout Extractor 🦀

[![codecov](https://codecov.io/gh/bartman/wxrust/branch/master/graph/badge.svg)](https://app.codecov.io/github/bartman/wxrust)

A Rust CLI tool to extract and display workouts from the WeightXReps.net website. It authenticates using credentials from a file and retrieves workout data via the GraphQL API, with support for listing, showing, summarizing, and fetching workouts into local cache.

## Features

- Authenticates using email and password from `credentials.txt` (email first line, password second)
- Authenticates and caches JWT token in `$XDG_CACHE_HOME/wxrust/token` (or `~/.cache/wxrust/token`)
- Retrieves individual workouts or lists of workout dates
- Formats output with colors matching the website editor
- Supports detailed views, summaries, and date filtering
- Handles JWT tokens and GraphQL queries efficiently
- Caches workout data locally in XDG cache directory for improved performance
- Fetches and caches workouts in bulk with progress indication
- Compares local cache with server versions
- Imports workouts from text export files

## Installation

Clone the repository and build with Cargo:

```bash
git clone https://github.com/bartman/wxrust.git
cd wxrust
cargo build --release
```

## Testing

Run unit tests with `cargo test`. For end-to-end smoke tests requiring credentials, use `./smoke.py`

To capture failures, if any, use this:
```bash
./smoke.py --work-dir tmp --output smoke.log
```

## Setup

Create a `credentials.txt` file with your WeightXReps account email on the first line and password on the second. The program looks for this file in the following locations (in order):

- The path specified with `--credentials` option (if provided)
- `$XDG_CONFIG_HOME/wxrust/credentials.txt` (or `~/.config/wxrust/credentials.txt` if `XDG_CONFIG_HOME` is not set)
- `~/.config/wxrust/credentials.txt`
- `./credentials.txt` (in the current working directory)

## Usage

### Global Options

- `--credentials <file>`: Path to credentials file (optional, falls back to standard locations)
- `--force-authentication`: Force re-login, ignore cached token (still saves new token to cache)
- `--no-network`: Skip network access, use cache only (workouts and dates come from local cache)
- `--no-cache`: Skip cache lookup, fetch all data from server (writes still happen unless `--no-cache-write` is also used)
- `--no-cache-write`: Disable cache writes (reads still happen unless `--no-cache` is also used)
- `--color <always|never|auto>`: Control color output (default: auto, based on TTY)

**Note:** `--no-network` and `--no-cache` are mutually exclusive.

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
- List in a date range: `wxrust list 2025-10-01..2025-10-31`
- Reverse order: `wxrust list --count 5 --reverse`
- List all (up to 1000): `wxrust list --all`

#### Fetch Workouts

- Fetch and cache workouts for 2025: `wxrust fetch 2025`
- Fetch all workouts: `wxrust fetch`
- Show diff between local and server: `wxrust fetch --diff 2025-10`
- Force re-download: `wxrust fetch --force 2025`
- Import from text export file: `wxrust fetch --file export.txt`

#### Table Command (PR Progression)

Display a progression table showing personal records (PRs) over time for specific exercises.

- Show PRs for all exercises: `wxrust table`
- Filter by exercise name: `wxrust table deadlift`
- Multiple filters (OR logic): `wxrust table deadlift squat`
- Filter by date range: `wxrust table 2025`
- Combine date and exercise filters: `wxrust table 2025 deadlift`

Arguments are automatically classified as dates or exercise filters:
- **Date formats**: `YYYY`, `YYYY-MM`, `YYYY.MM`, `YYYYMM`, `YYYY-MM-DD`, `YYYY.MM.DD`, `YYYYMMDD`
- **Everything else**: Treated as exercise name filter (case-insensitive substring match)

The table shows:
- Date of each PR, days ago, body weight
- Exercise name and estimated 1RM (Brzycki formula)
- Projected weights for rep ranges 1-10
- Color gradient from oldest (cool colors) to newest (warm colors)
 - Records within 7 days highlighted in bright yellow

#### Heatmap Command

Display a calendar heatmap showing workout intensity over time.

- Show 1RM heatmap: `wxrust heatmap`
- Show sets heatmap: `wxrust heatmap --sets`
- Show reps heatmap: `wxrust heatmap --reps`
- Show volume heatmap: `wxrust heatmap --volume`
- Show weight heatmap: `wxrust heatmap --weight`
- Filter by date range: `wxrust heatmap 2025`
- Filter by exercise: `wxrust heatmap deadlift`
- Combine filters: `wxrust heatmap 2025 deadlift --sets`

The heatmap displays a calendar grid with days colored by intensity (green gradient). Days without workouts are gray. Without color support, uses symbol gradients.

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

# List workouts in October 2025
wxrust list 2025-10-01..2025-10-31

# Fetch workouts for 2025
wxrust fetch 2025

# Show differences for October workouts
wxrust fetch --diff 2025-10

# Show PR progression table for deadlifts in 2025
wxrust table 2025 deadlift

# Show all PRs for squats
wxrust table squat

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
- `indicatif`: Progress bars
- `similar`: Text diffing

## API Details

Interacts with WeightXReps GraphQL API at `https://weightxreps.net/api/graphql`. Uses `login` mutation for auth, `jrange` query for date ranges, and `JDay` query for individual workouts. Supports efficient connection reuse for multiple requests.
