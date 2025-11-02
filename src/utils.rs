use chrono::{NaiveDate, Datelike};

pub fn parse_date_range(range: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let parts: Vec<&str> = range.split("..").collect();
    if parts.len() == 1 {
        let start = parse_date_boundary(range, false)?;
        let end = parse_date_boundary(range, true)?;
        Ok((start, end))
    } else if parts.len() == 2 {
        let start = parse_date_boundary(parts[0], false)?;
        let end = parse_date_boundary(parts[1], true)?;
        Ok((start, end))
    } else {
        Err("Invalid range format".to_string())
    }
}

// generate a string from text given.
// looks for dates like YYYYMMDD, YYYY/MM/DD, YYYY-MM-DD, or YYYY.MM.DD
// if DD is missing picks the first or last day of the month (depending on end)
// if MMDD is missing picks the first or last day of the year (depending on end)
// examples:
// "20250527"             -> NativeDate of 2025/05/27       (ignores end)
// "20250-05" end=false   -> NativeDate of 2025/05/01
// "20250-05" end=true    -> NativeDate of 2025/05/31
pub fn parse_date_boundary(s: &str, end: bool) -> Result<NaiveDate, String> {
    let mut parts = Vec::new();
    for p in s.split(['-', '/', '.']) {
        if !p.is_empty() {
            parts.push(p);
        }
    }

    let (year_str, month_str, day_str) = if parts.len() == 1 {
        let compact = parts[0];
        if compact.len() == 8 {
            // YYYYMMDD
            (compact[0..4].to_string(), compact[4..6].to_string(), compact[6..8].to_string())
        } else if compact.len() == 6 {
            // YYYYMM
            (compact[0..4].to_string(), compact[4..6].to_string(), "".to_string())
        } else if compact.len() == 4 {
            // YYYY
            (compact.to_string(), "".to_string(), "".to_string())
        } else {
            return Err("Invalid compact date format".to_string());
        }
    } else if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string(), "".to_string())
    } else if parts.len() == 3 {
        (parts[0].to_string(), parts[1].to_string(), parts[2].to_string())
    } else if parts.len() == 0 {
        return Err("Empty date string".to_string());
    } else {
        return Err("Too many parts".to_string());
    };

    let year: i32 = year_str.parse().map_err(|_| "Invalid year")?;
    if year_str.len() != 4 {
        return Err("Year must be 4 digits".to_string());
    }

    if month_str.is_empty() {
        return Ok(if end {
            NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
        });
    }

    let month: u32 = month_str.parse().map_err(|_| "Invalid month")?;
    if month == 0 || month > 12 {
        return Err("Invalid month".to_string());
    }

    if day_str.is_empty() {
        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - chrono::Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - chrono::Duration::days(1)
        }.day();

        let day = if end { last_day } else { 1 };
        return Ok(NaiveDate::from_ymd_opt(year, month, day).unwrap());
    }

    let day: u32 = day_str.parse().map_err(|_| "Invalid day")?;
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| "Invalid date".to_string())
}