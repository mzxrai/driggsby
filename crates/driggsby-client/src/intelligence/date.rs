use chrono::{Datelike, Duration, NaiveDate};

use crate::intelligence::types::IntelligenceFilter;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CadenceKind {
    Weekly,
    Biweekly,
    Monthly,
}

impl CadenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Biweekly => "biweekly",
            Self::Monthly => "monthly",
        }
    }

    pub const fn expected_interval_days(self) -> i64 {
        match self {
            Self::Weekly => 7,
            Self::Biweekly => 14,
            Self::Monthly => 30,
        }
    }

    pub fn advance(self, date: NaiveDate) -> NaiveDate {
        match self {
            Self::Weekly => date + Duration::days(7),
            Self::Biweekly => date + Duration::days(14),
            Self::Monthly => add_months_clamped(date, 1),
        }
    }
}

pub fn build_filter(
    from: Option<&str>,
    to: Option<&str>,
    command: &str,
) -> ClientResult<IntelligenceFilter> {
    let parsed_from = match from {
        Some(value) => Some(parse_iso_date_strict(value, "from", command)?),
        None => None,
    };
    let parsed_to = match to {
        Some(value) => Some(parse_iso_date_strict(value, "to", command)?),
        None => None,
    };

    if let (Some(start), Some(end)) = (parsed_from, parsed_to)
        && start > end
    {
        return Err(ClientError::invalid_argument_for_command(
            "Invalid date range: `from` must be on or before `to`.",
            Some(command),
        ));
    }

    Ok(IntelligenceFilter {
        from: parsed_from,
        to: parsed_to,
    })
}

pub fn format_iso_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn parse_transaction_date(value: &str) -> Option<NaiveDate> {
    if !looks_like_iso_date(value) {
        return None;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn add_months_clamped(date: NaiveDate, months: i32) -> NaiveDate {
    let current_month = i32::try_from(date.month()).unwrap_or(1);
    let mut raw_month = current_month + months;
    let mut year = date.year();

    while raw_month > 12 {
        raw_month -= 12;
        year += 1;
    }
    while raw_month < 1 {
        raw_month += 12;
        year -= 1;
    }

    let month_u32 = u32::try_from(raw_month).unwrap_or(1);
    let day = date.day().min(days_in_month(year, month_u32));
    if let Some(result) = NaiveDate::from_ymd_opt(year, month_u32, day) {
        return result;
    }
    date
}

fn parse_iso_date_strict(value: &str, field_name: &str, command: &str) -> ClientResult<NaiveDate> {
    if !looks_like_iso_date(value) {
        return Err(ClientError::invalid_argument_for_command(
            &format!("`{field_name}` must use YYYY-MM-DD format with a real calendar date."),
            Some(command),
        ));
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        ClientError::invalid_argument_for_command(
            &format!("`{field_name}` must use YYYY-MM-DD format with valid calendar values."),
            Some(command),
        )
    })
}

fn looks_like_iso_date(value: &str) -> bool {
    if value.len() != 10 {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    for index in [0usize, 1, 2, 3, 5, 6, 8, 9] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    true
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{CadenceKind, add_months_clamped, build_filter, format_iso_date};

    #[test]
    fn month_clamping_handles_end_of_month_transitions() {
        let jan_31 = NaiveDate::from_ymd_opt(2026, 1, 31);
        assert!(jan_31.is_some());
        if let Some(value) = jan_31 {
            let feb = add_months_clamped(value, 1);
            assert_eq!(format_iso_date(&feb), "2026-02-28");
            let mar = add_months_clamped(feb, 1);
            assert_eq!(format_iso_date(&mar), "2026-03-28");
        }
    }

    #[test]
    fn build_filter_rejects_invalid_ranges() {
        let result = build_filter(Some("2026-03-01"), Some("2026-02-01"), "recurring");
        assert!(result.is_err());
    }

    #[test]
    fn cadence_advance_uses_month_clamp() {
        let jan_31 = NaiveDate::from_ymd_opt(2026, 1, 31);
        assert!(jan_31.is_some());
        if let Some(value) = jan_31 {
            let next = CadenceKind::Monthly.advance(value);
            assert_eq!(format_iso_date(&next), "2026-02-28");
        }
    }
}
