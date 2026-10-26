# weightxreps.net GraphQL API (as used by wxrust)

Source of truth for the schema: `weightxreps-client/src/data/generated---db-types-and-hooks.tsx`
(generated from the server via `npm run exportschema`) plus
`generated---db-introspection.json`. The notes below cover only what
this project (`wxrust`) actually uses, with pointers to the fuller
site queries for context.

## Transport

- Single endpoint: `POST https://weightxreps.net/api/graphql`
  (`src/api.rs`: `ReqwestClient::login_request`, `graphql_request`).
  The site picks the URI from `REACT_APP_REMOTE_SERVER` in
  `weightxreps-client/src/data/db.js` (`$serverURI`): `staging` →
  `https://staging.weightxreps.net/api/graphql`, `production` →
  `https://weightxreps.net/api/graphql`, otherwise `/api/graphql`
  (prod build) or `http://localhost:4000/graphql` (dev).
- Body is JSON: `{ "query": "...", "variables": { ... } }`.
  Responses are `{ "data": {...} | null, "errors": [{ "message" }] | null }`
  (`src/models.rs`: `GraphQLResponse<T>`, `GraphQLError`).
- Authenticated calls add `Authorization: Bearer <token>`.
  The site injects it in an Apollo `authLink` from
  `localStorage.getItem('token')` (`session-handler.js`:
  `getAuthorizationHeaders`). wxrust sets the same header manually
  in `graphql_request` / `workout_request`. The login call itself is
  unauthenticated.
- HTTP client: shared `reqwest::Client` with
  `pool_max_idle_per_host(32)`, `tcp_nodelay`, gzip (`Cargo.toml`
  `reqwest` features `json`, `gzip`). Response body byte counts are
  tallied in `api::transfer_stats` for `fetch --stats`.

## Scalars

From the generated `Scalars` type:

| Scalar | Meaning |
|---|---|
| `YMD` | Calendar date `YYYY-MM-DD` (also `YYYYMMDD` exists). Workout-day key for `jday`/`jrange`. |
| `UTCDate` | Full timestamp (used by `UCard.when`, notifications, feed cursors — not by wxrust). |
| `ID` / `Int` / `Float` / `String` / `Boolean` | Standard. Note `uid` is passed as a **string** (`"2751"`), see `jrange` variables. |

## Authentication: `login`

Site mutation (`LoginDocument`):

```graphql
mutation Login($u: String!, $p: String!) {
  login(u: $u, p: $p)
}
```

- Returns a JWT **string** (`Mutation.login: String`). Siblings
  `loginWithGoogle` / `loginWithFirebase` exist but wxrust never uses them.
- wxrust (`src/auth.rs::login`): reads `credentials.txt` (email line 1,
  password line 2), posts `GraphQLRequest { query, variables: { u, p } }`,
  caches `{ token, uid, exp }` as JSON in the token file
  (`<cache>/wxrust/token`; `~/.config/wxrust/token` in older notes —
  now `get_cache_base_dir()/token`). Reuses the cached token until `exp`
  passes unless `-a/--force-authentication`.
- JWT payload claims (`Claims { id: u32, exp: u64 }`): `id` is the numeric
  user id (used as `uid` in every later query), `exp` is unix seconds.
  Decoded with base64 `URL_SAFE_NO_PAD` on segment 2. No signature
  verification — expiry check only.
- `--no-network` mode skips login and loads `uid` from the cached token
  (`load_uid_from_cache`); it errors if the cache is missing/expired.

## Session / user preferences: `getSession`

Site query (`GetSessionDocument`) fetches `getSession { user { ...UserFields } time forum {...} }`.
`UserFields` includes `id uname usekg isf bw cc slvl sok sleft age private
joined forumRole custom1RM est1RMFactor jranges estimate1RMFormula socialLinks ...`.

wxrust uses a minimal subset (`src/api.rs::get_user_info`), lazily and once
per run (cached in a `OnceCell` + `user_wants_kg` cache file):

```graphql
query {
  getSession {
    user {
      usekg
    }
  }
}
```

- `user.usekg`: `1` = kilograms, `0` = pounds. Drives weight parsing and
  display (`resolve_user_wants_kg`, `ParserOptions::new(user_wants_kg)`).
- Session is **not** fetched on every command — only when the unit
  preference is needed and not already in
  `<cache>/wxrust/user_wants_kg` (`"1\n"`/`"0\n"`). `write_cached_user_wants_kg`
  is called from `get_user_info` on success.

## Single workout: `jday`

Site query (`JDayDocument`, variables `QueryJdayArgs { uid: ID!, ymd: YMD }`):

```graphql
query JDay($uid: ID!, $ymd: YMD) {
  jday(uid: $uid, ymd: $ymd) {
    id log fromMobile bw
    eblocks { eid sets { ...SetFields ...WxDoTFields ...WxDoTFieldsExtras } }
    exercises { exercise { id name type } best { eff {...} int {...} prsWxDorT {...} } }
    utags { id name } utagsValues { ... }
  }
  getGoals { ... }
}
```

### What wxrust requests

`src/workouts.rs`: `JDAY_FIELDS` + `build_jday_query(uid, date)`:

```graphql
query {
  jday(uid: 2751, ymd: "2024-01-15") {
    log
    bw
    eblocks {
      eid
      sets { w r s lb rpe pr est1rm eff int type t d dunit speed force c }
    }
    exercises {
      exercise { id name type }
    }
  }
}
```

Deliberately dropped vs the site: `jday.id` (stable logid for
comments/likes), `fromMobile`, `exercises.best`, `utags`/`utagsValues`,
`getGoals`, `__typename`. wxrust deserializes into `models::JDay
{ log, bw, eblocks, exercises }` — extra server fields are ignored by serde.

### Batching with aliases

GraphQL aliases let one round-trip fetch many days
(`src/workouts.rs::build_batch_jday_query`, `JDAY_BATCH_SIZE = 10`):

```graphql
query {
  d0: jday(uid: 2751, ymd: "2024-01-15") { log bw eblocks { ... } exercises { ... } }
  d1: jday(uid: 2751, ymd: "2024-01-17") { log bw eblocks { ... } exercises { ... } }
}
```

Response is `HashMap<String, Option<JDay>>` (`BatchJDayData`); a `null`
alias value means "no workout that day". Up to `FETCH_CONCURRENCY = 8`
batch requests run concurrently (`get_jdays_with_callback` via
`futures::stream::buffer_unordered`), preserving input order on output.

### `JLog` shape (server type)

`JLog { id: ID!, log: String, fromMobile: Boolean, bw: Float,
eblocks: [EBlock], exercises: [ERef], utags: [UTag], utagsValues: [UTagValue] }`.
Note `id` is a comment/like key, **not** a version stamp — there is no
`updatedAt`, so wxrust cannot do mtime-based cache invalidation.

## Date listing: `jrange`

Site query (`GetJRangeDocument`, `QueryJrangeArgs { uid: ID!, ymd: YMD!, range: Int! }`):

```graphql
query GetJRange($uid: ID!, $ymd: YMD!, $range: Int!) {
  jrange(uid: $uid, ymd: $ymd, range: $range) {
    from to
    exercises { id name type }
    days { on did { eid sets { w r s lb ubw c rpe pr est1rm eff int ...WxDoTFields ...WxDoTFieldsExtras } } }
    utags { tags { id name automatic } values { tagid ymd type value } }
  }
}
```

### Semantics (important)

- `range` is **weeks**, max **32** (`JRANGE_MAX_WEEKS = 32`, server rejects
  more). Window covers `[ymd − range*7 days, ymd]`.
- The site itself uses this for the calendar zoom slider
  (`calendario-zoom-slider.js`), quick-range overview
  (`quick-range-overview.js`), and SBD sidebar — with the full
  `days.did` + `exercises` + `utags` payload.

### What wxrust requests

Minimal presence-only projection (`src/workouts.rs::JRANGE_QUERY`):

```graphql
query GetJRange($uid: ID!, $ymd: YMD!, $range: Int!) {
  jrange(uid: $uid, ymd: $ymd, range: $range) {
    days {
      on
    }
  }
}
```

with variables `{ "uid": "<id-as-string>", "ymd": "2024-01-15", "range": 32 }`.
`JRangeDayData` here is just `{ on: YMD }`; the `did` index column the site
uses is not fetched (wxrust re-fetches bodies via batched `jday`, which
includes the `log` text `jrange` lacks). `on` values are truncated to
`YYYY-MM-DD` (`ymd_from_jrange_on`).

Window splitting (`jrange_windows` / `fetch_jrange_windows`): a bounded
`[oldest, latest]` range is covered by concurrent `jrange` windows of ≤32
weeks walking **backwards** from `latest`, overlapping one day so boundary
workouts are not missed. Unbounded history (`-s -1`) still paginates
sequentially backwards (`get_dates` loop); bounded ranges are parallel.

### `-s/--scan-days` policy (`resolve_date_scan`)

| Flag | Behavior |
|---|---|
| `-s -1` | `DateScan::Full` — full-history `jrange` walk. |
| `-s 0` (default) | Scan `[last_cached_date, today]` and union with cache; `CacheOnly` if cache already reaches today; `Full` if cache disabled/empty. |
| `-s N` (N>0) | Hybrid scan of `[today−N, today]` ∪ cache. |
| `--no-network` | Flag ignored; cache only. |

This finds **new dates** only — edits to already-cached days are invisible
(no server mtime). `fetch --diff` / `--force` re-downloads to catch edits.

## Full export: `downloadLogs` (not used by fetch)

```graphql
query DownloadLogs {
  downloadLogs { ...jeditorDataFields }
}
```

Returns the current user's **entire** history as one `JEditorData`
(`etags, baseBW, exercises { e { id name type } days reps }, utags,
did: [JEditorBWTag | JEditorDayTag | JEditorEBlock | JEditorNewExercise |
JEditorText | UTagValue]`). The site's export widget
(`pages/download-logs-widget.js`) converts `did` rows to text client-side
(`convertJEditorData2Text(did, usekg, utags)` / `LogTextEditor.js`
`__convertJEditorDataToText`). wxrust intentionally does **not** use this
for `fetch`: converting `JEditorData` can diverge from the `jday` cache
format (`format_workout_for_cache` + trailing newline), so `fetch` uses
batched `jday` instead.

## Activity feed: `getActivityFeed` (not usable as history/mtime)

```graphql
query GetFeed($type: ActivityFeedType!, $olderThan: UTCDate, $newerThan: UTCDate) {
  getActivityFeed(type: $type, olderThan: $olderThan, newerThan: $newerThan) {
    user { ...BriefUserFields }
    when text workoutPreview { r w e { id name type } }
    andXmore posted media itemsLeftAfterThis
    utags { tags { id name } values { id tagid type value } }
  }
}
```

`UCard { when: UTCDate, posted: YMD, ... }` looks like a timestamp, but it is
the **social feed** (`following`/`global`), not a per-log mtime and not a
full history — so wxrust documents it only to explain why cache
invalidation can't use it (`JLog`/`JRangeDayData` carry no version).

## Entity reference (server types, abridged)

- `EBlock { eid: ID!, sets: [Set] }` — one exercise block; `eid` joins to `Exercise.id`.
- `Set` — weights/reps plus metadata. All nullable:
  `w r s lb ubw c rpe pr est1rm eff int type t d dunit speed force`.
  Fragments: `SetFields { w r s lb ubw c rpe pr est1rm eff int }`,
  `WxDoTFields { type t d dunit }` (time/distance sets),
  `WxDoTFieldsExtras { speed force }`.
  wxrust's `models::Set` keeps `w r s lb rpe c set_type usebw` and reads the
  WxDoT extras (`t d dunit speed force`, `int eff est1rm pr`) without
  interpreting most of them. `ubw` (server) ≈ `usebw` (wxrust model).
- `Exercise { id, name, type }` — `type` set ⟺ "official" exercise.
  `ERef { exercise, best }` wraps it per workout; wxrust drops `best`.
- `User` — see `getSession` above. `usekg: Int` (`1` kg / `0` lb) is the only
  field wxrust reads.
- `JRangeData { from: YMD, to: YMD, exercises: [Exercise], days: [JRangeDayData], utags }`;
  `JRangeDayData { on: YMD, did: [EBlock] }` (wxrust reads only `on`).

## wxrust operation → API map

| Operation | Code | Request |
|---|---|---|
| login | `auth::login` → `api::login_request` | `mutation login($u,$p){login}` — unauthenticated |
| unit pref | `api::ReqwestClient::get_user_info` | `query { getSession { user { usekg } } }` (lazy, cached) |
| one workout | `workouts::get_jday` | `build_jday_query` — single `jday(uid, ymd)` |
| many workouts | `get_jdays_batch` / `get_jdays_with_callback` | `build_batch_jday_query` — `d0..d9` aliases, 10/req, 8 concurrent |
| date list (default) | `get_dates` + `resolve_date_scan` | cache ∪ bounded `JRANGE_QUERY` (`days{on}` only) |
| date list (full) | `get_dates` loop / `fetch_jrange_windows` | `JRANGE_QUERY` with `range ≤ 32` |
| fetch/diff | `fetch::fetch_command` | reuses batched `jday`; `fetch --diff` compares raw cache text (`read_cached_jday_text`) vs `format_cached_jday_text` |
| import | `fetch_from_file` | no network — parses a text export (`parse_file_export`) |

Error shape throughout: GraphQL `errors[].message` joined with `"; "`
(`get_jday`, `fetch_jrange`, `login`). `-v/--verbose` logs the query +
variables (blue) and raw response body (green/red) to stderr
(`log_verbose_request` / `log_verbose_response`).

## What wxrust deliberately does not use

- `jeditor` / `saveJEditor`, `execExercise`, goals, forum, inbox, AI review,
  achievements, community stats, `getExercises`, `getCalendarDays`,
  `alsoposted`, `loginWithGoogle`/`loginWithFirebase` — present in the
  generated hooks but no wxrust call site.
- `jrange.exercises/did/utags`, `jday.best/utags/getGoals` — fetched by the
  site's journal UI (`jday.js`, `quick-range-overview.js`,
  `SBDRankSidebar.js`) but dropped from wxrust projections to keep the
  cache byte-identical to `format_workout_for_cache` output.
