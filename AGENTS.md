# AGENTS.md - WeightXReps Rust Client Development Session

## Session Summary

This session developed a Rust program to authenticate with WeightXReps (https:weightxreps.net/) and retrieve user workouts. The program logs in using email/password from a file,
decodes the JWT token for user ID, queries the GraphQL API for workout data, and formats the output to match the site's log format.

## Instructions from user

- make sure code changes build with `cargo build`
- make sure code changes pass unit tests `cargo test`
- when generating new code, consider splitting new functionality into helper functions
- write unit tests for new functions; put unit tests into `tests/test_*.rs`
- update README.md when new features are added
- update AGENTS.md when you discover new information that is useful to future agents

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
- **Performance**: Reuse HTTP client across requests to maintain connection pooling and avoid TCP overhead. Implemented concurrent API fetching for multiple workouts and session-level caching of user preferences to reduce latency and redundant calls.
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

You can look in `weightxreps-client/src/data/generated---db-types-and-hooks.tsx` for API information.

## Program Features

- Reads credentials from `credentials.txt` (email first line, password second).
- Authenticates and caches JWT token in `~/.config/wxrust/token`.
- Decodes token to extract user ID.
- Queries individual workouts (`JDay`) or date ranges (`jrange`).
- Formats structured JSON data into human-readable text with compression and colors.
- Supports listing dates, detailed views, and summaries with concurrent fetching for improved performance.
- Outputs full workout logs matching site format (date, bodyweight, program, sets, URL).
- Handles user unit preferences for body weight display (kg or lb) fetched from API.
- CLI with subcommands, options for count, before, reverse, details, summary, color control, and date ranges (using .. separator).
- Ensures ordered output in list commands by buffering concurrent async requests to maintain sequence.

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
- **Code Coverage**: 80.25% overall coverage (256/319 lines) using `cargo-tarpaulin` after excluding API and main functions. High coverage in core modules (formatters: 83%, auth: 76%, workouts: 63%). HTML reports generated in CI via `coverage.sh` script, into `coverage/` directory.
- **CI/CD**: GitHub Actions workflow at https://github.com/bartman/wxr-rs/actions/workflows/ci.yml that builds, tests, and generates coverage reports on every push/PR to master. Coverage dashboard at https://app.codecov.io/github/bartman/wxr-rs.
- **Smoke Tests**: Python script `smoke.py` for end-to-end testing requiring credentials. Tests run executable with expected outputs, supporting flags for case, blank lines, and whitespace insensitivity. Includes options for listing tests (`--list`), running specific tests (`--test <name>`), and preserving work directories (`--keep-work-dir`). Captures stdout, stderr, and return codes in work directories for inspection. Example tests for `--help`, show commands, and list commands.

### Smoke Test Script Details

`smoke.py` is a Python script located in the project root that automates running smoke tests for the `wxrust` binary. It executes commands defined in test directories under `smoke/`, compares outputs to expected results, and reports pass/fail status.

#### Test Directory Structure

Each test is a subdirectory under `smoke/` (e.g., `000-help/`), containing:

- `command`: Shell command to execute (with variable substitution).
- `expected.stdout`: Expected standard output.
- `expected.stderr`: Expected standard error (optional).
- `expected.code`: Expected return code (optional).
- `flags`: Key=value file for comparison flags (e.g., `ignore-case=true`).

#### Command Line Options

- `--target-dir <path>`: Build directory (default: `target`).
- `--smoke-dir <path>`: Smoke tests directory (default: `smoke`).
- `--work-dir <path>`: Temporary work directory (default: auto-generated `/tmp/{project}-{pid}`).
- `--output <file>`: File for verbose logs (default: none, silenced).
- `--keep-work-dir`: Keep work directory after tests (default: delete if auto-generated).
- `--list`: List all available test names.
- `--test <name>`: Run only the specified test.
- `--variable <var>=<val>`: Override variables.
- `--variables`: List all variables and their values.

#### Variable Substitution

Commands support `{{VARIABLE}}` placeholders. Predefined variables include `PROJECT_NAME`, `PID`, `TARGET_DIR`, `TARGET`, `PROGRAM`, `SMOKE_DIR`, `CREDENTIALS`, `WORK_DIR`, `PROGRAM_PATH`.

#### Comparison Flags

- `ignore-case=true`: Case-insensitive comparison.
- `ignore-blank-lines=true`: Ignore blank lines.
- `ignore-white-space=true`: Normalize whitespace.

#### Output and Logging

- Test results printed to stdout with color (PASS green, FAIL red).
- On failure, diff written to verbose log; expected and actual file paths printed for manual diffing.
- Actual outputs saved in `WORK_DIR/TEST_NAME/` as `output.stdout`, `output.stderr`, `output.code`.

#### Usage Examples

- List tests: `python3 smoke.py --list`
- Run specific test: `python3 smoke.py --test 000-help`
- Run all with verbose log: `python3 smoke.py --output smoke.log`

## Future Improvements

- Add support for year/month range queries.
- Implement caching for workout data.
- Add export options (JSON, CSV).
- Support for user profile and goals queries.
- Enhance error handling and retry logic.


