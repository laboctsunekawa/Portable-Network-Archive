use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    ops::{Add, Sub},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum DateTimeError {
    #[error("Failed to parse seconds since unix epoch")]
    ParseError,
    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),
    #[error(transparent)]
    ParseDatetimeError(#[from] parse_datetime::ParseDateTimeError),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum DateTime {
    Naive(chrono::NaiveDateTime),
    Zoned(chrono::DateTime<chrono::FixedOffset>),
    Date(chrono::NaiveDate),
    /// Unix epoch timestamp represented as a signed [`Duration`].
    Epoch {
        /// `true` if the timestamp is before the Unix epoch.
        negative: bool,
        /// Absolute time difference from the Unix epoch.
        duration: Duration,
    },
}

impl DateTime {
    #[inline]
    pub(crate) fn to_system_time(&self) -> SystemTime {
        fn from_timestamp(seconds: i64, nanos: u32) -> SystemTime {
            let duration = Duration::from_secs_f64(
                seconds.unsigned_abs() as f64 + f64::from(nanos) / 1_000_000_000.0,
            );
            if seconds < 0 {
                UNIX_EPOCH.sub(duration)
            } else {
                UNIX_EPOCH.add(duration)
            }
        }
        match self {
            Self::Naive(naive) => {
                // FIXME: Avoid `.unwrap()` call, use match statement with return Result.
                let naive = naive.and_local_timezone(chrono::Local).unwrap();
                from_timestamp(naive.timestamp(), naive.timestamp_subsec_nanos())
            }
            Self::Zoned(zoned) => from_timestamp(zoned.timestamp(), zoned.timestamp_subsec_nanos()),
            Self::Date(date) => {
                from_timestamp(date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp(), 0)
            }
            Self::Epoch { negative, duration } => {
                if *negative {
                    UNIX_EPOCH.sub(*duration)
                } else {
                    UNIX_EPOCH.add(*duration)
                }
            }
        }
    }
}

impl Display for DateTime {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Naive(naive) => Display::fmt(naive, f),
            Self::Zoned(zoned) => Display::fmt(zoned, f),
            Self::Date(date) => Display::fmt(date, f),
            Self::Epoch { negative, duration } => {
                let sign = if *negative { "-" } else { "" };
                let secs = duration.as_secs();
                let nanos = duration.subsec_nanos();
                if nanos == 0 {
                    write!(f, "@{}{}", sign, secs)
                } else {
                    write!(f, "@{}{}.{:09}", sign, secs, nanos)
                }
            }
        }
    }
}

impl FromStr for DateTime {
    type Err = DateTimeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(seconds) = s.strip_prefix('@') {
            // GNU tar allows both comma and dot as decimal separators
            let seconds_str = if seconds.contains(',') {
                Cow::Owned(seconds.replace(',', "."))
            } else {
                Cow::Borrowed(seconds)
            };
            let value = f64::from_str(&seconds_str).map_err(|_| DateTimeError::ParseError)?;
            let negative = value.is_sign_negative();
            let duration = Duration::from_secs_f64(value.abs());
            Ok(Self::Epoch { negative, duration })
        } else if let Ok(naive) = chrono::NaiveDateTime::from_str(s) {
            Ok(Self::Naive(naive))
        } else if let Ok(naive_date) = chrono::NaiveDate::from_str(s) {
            Ok(Self::Date(naive_date))
        } else {
            parse_datetime::parse_datetime(s)
                .map_err(Into::into)
                .map(Self::Zoned)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_parse_valid() {
        let valid_dt = "2024-03-20T12:34:56";
        let datetime = DateTime::from_str(valid_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56");
    }

    #[test]
    fn test_datetime_parse_with_timezone() {
        let zoned_dt = "2024-03-20T12:34:56+09:00";
        let datetime = DateTime::from_str(zoned_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56 +09:00");
        let zoned_dt = "2024-03-20T12:34:56Z";
        let datetime = DateTime::from_str(zoned_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56 +00:00");
    }

    #[test]
    fn test_datetime_parse_invalid() {
        let invalid_dt = "invalid-datetime";
        assert!(DateTime::from_str(invalid_dt).is_err());
    }

    #[test]
    fn test_to_system_time_after_epoch() {
        let positive_dt = "2024-03-20T12:34:56Z";
        let datetime = DateTime::from_str(positive_dt).unwrap();
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn test_to_system_time_before_epoch() {
        let negative_dt = "1969-12-31T23:59:59Z";
        let datetime = DateTime::from_str(negative_dt).unwrap();
        let system_time = datetime.to_system_time();
        assert!(system_time < UNIX_EPOCH);
    }

    #[test]
    fn test_relative_time_format_positive() {
        let datetime = DateTime::from_str("@1234567890").unwrap();
        assert_eq!(datetime.to_string(), "@1234567890");
    }

    #[test]
    fn test_relative_time_format_negative() {
        let datetime = DateTime::from_str("@-1234567890").unwrap();
        assert_eq!(datetime.to_string(), "@-1234567890");
    }

    #[test]
    fn test_relative_time_format_decimal_dot() {
        let datetime = DateTime::from_str("@123.456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456000000");
    }

    #[test]
    fn test_relative_time_format_decimal_comma() {
        let datetime = DateTime::from_str("@123,456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456000000");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_dot() {
        let datetime = DateTime::from_str("@-123.456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456000000");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_comma() {
        let datetime = DateTime::from_str("@-123,456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456000000");
    }

    #[test]
    fn test_relative_time_format_zero() {
        let datetime = DateTime::from_str("@0").unwrap();
        assert_eq!(datetime.to_string(), "@0");
    }

    #[test]
    fn test_relative_time_format_negative_one() {
        let datetime = DateTime::from_str("@-1").unwrap();
        assert_eq!(datetime.to_string(), "@-1");
    }

    #[test]
    fn test_datetime_parse_and_display_date() {
        let datetime = DateTime::from_str("2024-04-01").unwrap();
        assert_eq!(datetime.to_string(), "2024-04-01");
    }

    #[test]
    fn test_to_system_time_naive() {
        let naive = chrono::NaiveDate::from_ymd_opt(2024, 4, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let datetime = DateTime::Naive(naive);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_date() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        let datetime = DateTime::Date(date);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_epoch() {
        let datetime = DateTime::Epoch {
            negative: false,
            duration: Duration::from_secs(1_234_567_890),
        };
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_epoch_with_nanos() {
        let datetime = DateTime::from_str("@1.5").unwrap();
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }
}
