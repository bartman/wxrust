
# AGENTS.md - WeightXReps Rust Client Development Session

## Session Summary

This session developed a Rust program to authenticate with WeightXReps (https:weightxreps.net/) and retrieve user workouts. The program logs in using email/password from a file,
decodes the JWT token for user ID, queries the GraphQL API for workout data, and formats the output to match the site's log format.

## Key Learnings

- **API Structure**: WeightXReps uses GraphQL at `/api/graphql` for all data operations. Authentication via JWT tokens in Authorization headers.
- **Authentication**: Login mutation `login(u: $u, p: $p)` returns a JWT token containing user ID in the `id` field.
- **Data Retrieval**: `JDay` query fetches workouts by user ID and date (YMD format: YYYY-MM-DD). Includes structured data like eblocks (exercise blocks), sets, exercises, and a
pre-formatted log.
- **Data Formatting**: Workout logs are formatted text with #exercise prefixes and compressed set notations (e.g., 135x5x3 or 445x1,3). Formatting logic found in client code.
- **Rust Implementation**: Used reqwest for HTTP, serde for JSON, base64 for JWT decoding. Handled GraphQL responses, error checking, and data transformation.

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
- Authenticates and retrieves JWT token.
- Decodes token to extract user ID.
- Queries latest workout for a specific date.
- Formats structured JSON data into human-readable text with compression.
- Outputs full workout log matching site format (date, bodyweight, program, sets, URL).

## Dependencies Added

- `reqwest` (0.12) with JSON features for HTTP requests.
- `serde` (1.0) with derive for JSON serialization/deserialization.
- `base64` (0.21) for JWT payload decoding.
- `tokio` (1) for async runtime.

## Future Improvements

- Handle multiple dates or latest workout dynamically.
- Add error recovery for network issues.
- Implement full compression logic for all set types.
- Support for other GraphQL queries (goals, user profile).

