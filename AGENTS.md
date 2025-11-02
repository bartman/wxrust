# AGENTS.md - WeightXReps Rust Client Development Session

## Session Summary

This session developed a Rust program to authenticate with WeightXReps (https:weightxreps.net/) and retrieve user workouts. The program logs in using email/password from a file,
decodes the JWT token for user ID, queries the GraphQL API for workout data, and formats the output to match the site's log format.

## Recent Updates

- **User Unit Preferences**: Implemented support for displaying body weight in the user's preferred units (kg or lb) based on API data.
  - Fixed GraphQL query from `userBasicInfo` (which returned null for `usekg`) to `getSession { user { usekg } }` to fetch preferences (`usekg`: 1=kg, 0=lb).
  - Updated `User` struct in `src/models.rs`: changed `usekg` from `bool` to `Option<i32>`.
  - Added `SessionInfo` struct for query response handling.
  - Modified unit conversion in `src/workouts.rs`: converts body weight from kg to lb only if `usekg != 1`; defaults to kg if data missing.
  - Updated tests in `tests/test_api.rs`, `tests/test_auth_integration.rs`, `tests/test_workouts.rs` for new mocks, expectations, and trait methods.
  - Feature now works: body weight displays correctly in preferred units via `cargo run -- show`.

## Key Learnings

- **API Structure**: WeightXReps uses GraphQL at `/api/graphql` for all data operations. Authentication via JWT tokens in Authorization headers.
- **Authentication**: Login mutation `login(u: $u, p: $p)` returns a JWT token containing user ID in the `id` field.
- **Data Retrieval**:
  - `JDay` query fetches workouts by user ID and date (YMD format: YYYY-MM-DD). Includes structured data like eblocks (exercise blocks), sets, exercises, and a pre-formatted log.
  - `jrange` query fetches a range of workout days around a given date, with configurable count (max 32). Returns days with workouts in the range.
- **Data Formatting**: Workout logs are formatted text with #exercise prefixes and compressed set notations (e.g., 135x5x3 or 445x1,3). Formatting logic found in client code.
- **Color Scheme**: Website editor uses specific RGB colors for syntax highlighting:
  - Date: #9D4EDD (157,78,221)
  - Body weight: #3A86FF (58,134,255)
  - Exercise names: #0096FF (0,150,255) - brighter blue for visibility
  - Weights: #FF7900 (255,121,0)
  - Reps: #00BBF9 (0,187,249)
  - Sets: #F15BB5 (241,91,181)
- **Performance**: Reuse HTTP client across requests to maintain connection pooling and avoid TCP overhead.
- **Rust Implementation**: Used reqwest for HTTP with client reuse, serde for JSON, base64 for JWT decoding, ansi_term for colors, atty for TTY detection. Handled GraphQL responses, error checking, and inline color application during text generation.

## Referenced Links

- **https:weightxreps.net/**: Main site for WeightXReps fitness tracking. Used for API endpoints and authentication.
- **https:github.com/bandinopla/weightxreps-client**: Public repository for the WeightXReps web client. Source of API details, GraphQL queries, and formatting logic.

## API Details Source

All API details (endpoints, queries, mutations, response structures) were discovered by searching the client codebase at https:github.com/bandinopla/weightxreps-client using
automated tools. Key files examined:

- GraphQL queries/mutations in JavaScript/TypeScript files.
- Authentication flow in login components.
- Data formatting in `src/codemirror/LogTextEditor.js` for log generation.
- No separate API documentation; details inferred from client-server interactions.

You can clone the weightxreps-client code, in the project directory, to inspect it and find out how to do things...

```
git clone https://github.com/bandinopla/weightxreps-client.git weightxreps-client
```

## Program Features

- Reads credentials from `credentials.txt` (email first line, password second).
- Authenticates and caches JWT token.
- Decodes token to extract user ID.
- Queries individual workouts (`JDay`) or date ranges (`jrange`).
- Formats structured JSON data into human-readable text with compression and colors.
- Supports listing dates, detailed views, and summaries.
- Outputs full workout logs matching site format (date, bodyweight, program, sets, URL).
- Handles user unit preferences for body weight display (kg or lb) fetched from API.
- CLI with subcommands, options for count, before, reverse, details, summary, color control, and date ranges (using .. separator).

## Dependencies Added

- `reqwest` (0.12) with JSON features for HTTP requests and client reuse.
- `serde` (1.0) with derive for JSON serialization/deserialization.
- `base64` (0.21) for JWT payload decoding.
- `tokio` (1) for async runtime.
- `clap` (4.0) for CLI parsing.
- `regex` (1.0) for text processing.
- `chrono` (0.4) for date handling.
- `ansi_term` (0.12) for terminal colors.
- `atty` (0.2) for TTY detection.
- `lazy_static` (1.4) for global state.
- `async-trait` (0.1) for async traits.
- `mockall` (0.12) for mocking in tests (dev-dependency).
- `tempfile` (3.0) for temporary files in tests (dev-dependency).

## Testing and CI/CD

- **Unit Tests**: Comprehensive test suite with 21 tests covering formatters, auth, workouts, and API stubbing. Tests are standalone, no external dependencies.
- **Integration Tests**: Stubbed API calls using `mockall` for testing authentication and data retrieval without real network access.
- **Code Coverage**: 54.08% overall coverage (179/331 lines) using `cargo-tarpaulin`. High coverage in core modules (formatters: 83%, auth: 76%, workouts: 63%). HTML reports generated in CI.
- **CI/CD**: GitHub Actions workflow at https://github.com/bartman/wxr-rs/actions/workflows/ci.yml that builds, tests, and generates coverage reports on every push/PR to master. Coverage dashboard at https://app.codecov.io/github/bartman/wxr-rs.

## Future Improvements

- Add support for year/month range queries.
- Implement caching for workout data.
- Add export options (JSON, CSV).
- Support for user profile and goals queries.
- Enhance error handling and retry logic.


