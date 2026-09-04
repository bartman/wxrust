use chrono::{Duration, NaiveDate};
use wxrust::utils::{parse_date_boundary, parse_date_range};
use wxrust::workouts::{jrange_windows, resolve_date_scan, DateScan, JRANGE_MAX_WEEKS};

fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn test_parse_date_boundary_full_date() {
    // Full date ignores the end parameter
    let date = parse_date_boundary("2025-05-27", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());

    let date = parse_date_boundary("2025-05-27", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());

    // Test with different separators
    let date = parse_date_boundary("2025/05/27", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());

    let date = parse_date_boundary("2025.05.27", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());
}

#[test]
fn test_parse_date_boundary_compact_yyyymmdd() {
    let date = parse_date_boundary("20250527", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());

    let date = parse_date_boundary("20250527", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());
}

#[test]
fn test_parse_date_boundary_month_only_end_false() {
    let date = parse_date_boundary("2025-05", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());

    // Compact
    let date = parse_date_boundary("202505", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
}

#[test]
fn test_parse_date_boundary_month_only_end_true() {
    let date = parse_date_boundary("2025-05", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());

    // Compact
    let date = parse_date_boundary("202505", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());

    // December
    let date = parse_date_boundary("2025-12", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
}

#[test]
fn test_parse_date_boundary_year_only_end_false() {
    let date = parse_date_boundary("2025", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

    // Compact
    let date = parse_date_boundary("2025", false).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
}

#[test]
fn test_parse_date_boundary_year_only_end_true() {
    let date = parse_date_boundary("2025", true).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
}

#[test]
fn test_parse_date_boundary_invalid() {
    // Invalid year length
    assert!(parse_date_boundary("202", false).is_err());

    // Invalid month
    assert!(parse_date_boundary("2025-13", false).is_err());

    // Invalid day
    assert!(parse_date_boundary("2025-05-32", false).is_err());

    // Invalid compact - too long
    assert!(parse_date_boundary("202511220", false).is_err());

    // Invalid compact - missing day digit
    assert!(parse_date_boundary("2024112", false).is_err());

    // Invalid compact - missing month digit
    assert!(parse_date_boundary("20231", false).is_err());

    // Invalid compact - missing year digit
    assert!(parse_date_boundary("202", false).is_err());

    // Invalid compact
    assert!(parse_date_boundary("2025052", false).is_err());

    // Too many parts - poorly separated
    assert!(parse_date_boundary("2-0-2-5-1-1-2-2", false).is_err());

    // Poorly ordered - DD/MM/YYYY
    assert!(parse_date_boundary("31/11/2024", false).is_err());

    // Out of range month
    assert!(parse_date_boundary("2025/31/11", false).is_err());

    // Too many parts
    assert!(parse_date_boundary("2025-05-27-01", false).is_err());

    // Empty
    assert!(parse_date_boundary("", false).is_err());
}

#[test]
fn test_parse_date_range_single_date() {
    let (start, end) = parse_date_range("2025-05-27").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());

    // Compact
    let (start, end) = parse_date_range("20250527").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 27).unwrap());
}

#[test]
fn test_parse_date_range_with_separator() {
    let (start, end) = parse_date_range("2025-05-01..2025-05-31").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());

    // Different separators
    let (start, end) = parse_date_range("2025/05/01..2025/05/31").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());
}

#[test]
fn test_parse_date_range_compact() {
    let (start, end) = parse_date_range("20250501..20250531").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());
}

#[test]
fn test_parse_date_range_month_range() {
    let (start, end) = parse_date_range("2025-05").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 5, 31).unwrap());
}

#[test]
fn test_parse_date_range_year_range() {
    let (start, end) = parse_date_range("2025").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(end, NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
}

#[test]
fn test_parse_date_range_open_end() {
    let today = chrono::Utc::now().date_naive();
    let (start, end) = parse_date_range("2025..").unwrap();
    assert_eq!(start, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(end, today);
}

#[test]
fn test_parse_date_range_invalid() {
    // Invalid date
    assert!(parse_date_range("invalid").is_err());

    // Too many range entries
    assert!(parse_date_range("20251122..20251122..20251122").is_err());

    // Too many parts
    assert!(parse_date_range("2025-05-01..2025-05-31..extra").is_err());
}

#[test]
fn test_jrange_windows_same_day() {
    let day = NaiveDate::from_ymd_opt(2023, 10, 1).unwrap();
    let windows = jrange_windows(day, day);
    assert_eq!(windows, vec![("2023-10-01".to_string(), 1)]);
}

#[test]
fn test_jrange_windows_empty_when_inverted() {
    let oldest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
    let latest = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    assert!(jrange_windows(oldest, latest).is_empty());
}

#[test]
fn test_jrange_windows_one_max_window() {
    let latest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
    let oldest = latest - chrono::Duration::days(JRANGE_MAX_WEEKS as i64 * 7);
    let windows = jrange_windows(oldest, latest);
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].0, "2026-09-04");
    assert_eq!(windows[0].1, JRANGE_MAX_WEEKS);
}

#[test]
fn test_jrange_windows_year_needs_two() {
    let oldest = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let latest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
    let windows = jrange_windows(oldest, latest);
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0], ("2026-09-04".to_string(), JRANGE_MAX_WEEKS));
    assert!(windows[1].1 >= 1 && windows[1].1 <= JRANGE_MAX_WEEKS);
    // Windows overlap by one day at the first window's start.
    let first_start = latest - chrono::Duration::days(JRANGE_MAX_WEEKS as i64 * 7);
    assert_eq!(windows[1].0, first_start.format("%Y-%m-%d").to_string());
}

#[test]
fn test_resolve_date_scan_negative_is_full() {
    let today = ymd(2026, 9, 4);
    assert_eq!(
        resolve_date_scan(-1, true, Some(today), today, None, None),
        DateScan::Full
    );
}

#[test]
fn test_resolve_date_scan_zero_without_cache_is_full() {
    let today = ymd(2026, 9, 4);
    assert_eq!(
        resolve_date_scan(0, false, Some(today), today, None, None),
        DateScan::Full
    );
    assert_eq!(
        resolve_date_scan(0, true, None, today, None, None),
        DateScan::Full
    );
}

#[test]
fn test_resolve_date_scan_zero_skips_when_current() {
    let today = ymd(2026, 9, 4);
    assert_eq!(
        resolve_date_scan(0, true, Some(today), today, None, None),
        DateScan::CacheOnly
    );
}

#[test]
fn test_resolve_date_scan_zero_since_last_cached() {
    let today = ymd(2026, 9, 4);
    let last = ymd(2026, 8, 20);
    assert_eq!(
        resolve_date_scan(0, true, Some(last), today, None, None),
        DateScan::Hybrid { oldest: last, latest: today }
    );
}

#[test]
fn test_resolve_date_scan_seven_days() {
    let today = ymd(2026, 9, 4);
    assert_eq!(
        resolve_date_scan(7, true, Some(today), today, None, None),
        DateScan::Hybrid {
            oldest: today - Duration::days(7),
            latest: today,
        }
    );
}

#[test]
fn test_resolve_date_scan_seven_without_cache() {
    let today = ymd(2026, 9, 4);
    assert_eq!(
        resolve_date_scan(7, false, None, today, None, None),
        DateScan::Hybrid {
            oldest: today - Duration::days(7),
            latest: today,
        }
    );
}

#[test]
fn test_resolve_date_scan_window_misses_historical_range() {
    let today = ymd(2026, 9, 4);
    let last = ymd(2026, 9, 1);
    assert_eq!(
        resolve_date_scan(
            0,
            true,
            Some(last),
            today,
            Some(ymd(2020, 1, 1)),
            Some(ymd(2020, 12, 31)),
        ),
        DateScan::CacheOnly
    );
}

#[test]
fn test_resolve_date_scan_intersects_requested_range() {
    let today = ymd(2026, 9, 4);
    let last = ymd(2026, 1, 15);
    assert_eq!(
        resolve_date_scan(
            0,
            true,
            Some(last),
            today,
            Some(ymd(2026, 1, 1)),
            Some(ymd(2026, 12, 31)),
        ),
        DateScan::Hybrid {
            oldest: last,
            latest: today,
        }
    );
}
