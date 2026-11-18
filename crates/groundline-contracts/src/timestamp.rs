//! The timestamp precision and syntax used by persisted GroundLine records.

use std::cmp::Ordering;
use std::ops::{Add, Sub};
use std::str::FromStr;

use time::error::{Parse, ParseFromDescription};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

pub use time::{Date, Duration, Month};

const NANOS_PER_SECOND: u32 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug)]
pub struct Timestamp {
    local: OffsetDateTime,
    // Preserve the separate second and its fractional part in legacy records.
    leap_second: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum TimestampFormat {
    Auto,
    Seconds,
    Millis,
    Micros,
}

impl Timestamp {
    pub fn now() -> Self {
        Self::regular(OffsetDateTime::now_utc())
    }

    fn regular(local: OffsetDateTime) -> Self {
        Self {
            local,
            leap_second: false,
        }
    }

    pub fn from_timestamp(seconds: i64, nanos: u32) -> Option<Self> {
        let leap_second = nanos >= NANOS_PER_SECOND;
        if leap_second && (nanos >= 2 * NANOS_PER_SECOND || seconds.rem_euclid(60) != 59) {
            return None;
        }
        let local = OffsetDateTime::from_unix_timestamp(seconds)
            .ok()?
            .replace_nanosecond(nanos % NANOS_PER_SECOND)
            .ok()?;
        Some(Self { local, leap_second })
    }

    pub fn parse_rfc3339(value: &str) -> Result<Self, Parse> {
        if !matches!(value.as_bytes().get(10), Some(b'T' | b't' | b' ')) {
            return Err(ParseFromDescription::InvalidComponent("separator").into());
        }
        let leap_second = value.get(17..19) == Some("60");
        // Older records also accept the Unicode minus sign in the UTC offset.
        let unicode_minus = value.len().checked_sub("−00:00".len()).filter(|&index| {
            index >= 19 && value.get(index..).is_some_and(|tail| tail.starts_with('−'))
        });
        let local = if leap_second || unicode_minus.is_some() {
            let mut input = value.to_owned();
            if let Some(index) = unicode_minus {
                input.replace_range(index..index + '−'.len_utf8(), "-");
            }
            if leap_second {
                input.replace_range(17..19, "59");
            }
            OffsetDateTime::parse(&input, &Rfc3339)?
        } else {
            OffsetDateTime::parse(value, &Rfc3339)?
        };
        Ok(Self { local, leap_second })
    }

    pub fn to_utc(self) -> Self {
        Self {
            local: self.local.to_offset(UtcOffset::UTC),
            ..self
        }
    }

    pub fn unix_timestamp(self) -> i64 {
        self.local.unix_timestamp()
    }

    pub fn unix_timestamp_millis(self) -> i64 {
        self.unix_timestamp() * 1_000 + i64::from(self.nanosecond() / 1_000_000)
    }

    pub fn nanosecond(self) -> u32 {
        self.local.nanosecond() + u32::from(self.leap_second) * NANOS_PER_SECOND
    }

    pub fn to_rfc3339(self) -> String {
        self.format_rfc3339(TimestampFormat::Auto, false)
    }

    pub fn format_rfc3339(self, precision: TimestampFormat, use_z: bool) -> String {
        let year = self.local.year();
        let year = if (0..=9999).contains(&year) {
            format!("{year:04}")
        } else {
            format!("{year:+05}")
        };
        let month = u8::from(self.local.month());
        let day = self.local.day();
        let (hour, minute, second) = self.local.to_hms();
        let second = second + u8::from(self.leap_second);
        let nanos = self.local.nanosecond();
        let fraction = match precision {
            TimestampFormat::Seconds => String::new(),
            TimestampFormat::Millis => format!(".{:03}", nanos / 1_000_000),
            TimestampFormat::Micros => format!(".{:06}", nanos / 1_000),
            TimestampFormat::Auto if nanos == 0 => String::new(),
            TimestampFormat::Auto if nanos.is_multiple_of(1_000_000) => {
                format!(".{:03}", nanos / 1_000_000)
            }
            TimestampFormat::Auto if nanos.is_multiple_of(1_000) => {
                format!(".{:06}", nanos / 1_000)
            }
            TimestampFormat::Auto => format!(".{nanos:09}"),
        };
        let offset = self.local.offset().whole_minutes();
        let zone = if use_z && offset == 0 {
            "Z".to_owned()
        } else {
            let sign = if offset < 0 { '-' } else { '+' };
            let offset = offset.unsigned_abs();
            format!("{sign}{:02}:{:02}", offset / 60, offset % 60)
        };
        format!("{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{fraction}{zone}")
    }

    pub fn checked_add(self, mut duration: Duration) -> Option<Self> {
        if self.leap_second {
            let nanos = i128::from(self.nanosecond()) + duration.whole_nanoseconds();
            let second = i128::from(NANOS_PER_SECOND);
            if duration.whole_seconds() == 0 && nanos < 2 * second {
                let local = self
                    .local
                    .replace_nanosecond((nanos % second) as u32)
                    .ok()?;
                return Some(Self {
                    local,
                    leap_second: nanos >= second,
                });
            }
            // Crossing backwards includes the extra second in this minute.
            if duration.is_negative() {
                duration = duration.checked_add(Duration::SECOND)?;
            }
        }
        self.local.checked_add(duration).map(Self::regular)
    }

    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        self.checked_add(Duration::ZERO.checked_sub(duration)?)
    }
}

impl FromStr for Timestamp {
    type Err = Parse;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_rfc3339(value)
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Timestamp {}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.unix_timestamp(), self.nanosecond())
            .cmp(&(other.unix_timestamp(), other.nanosecond()))
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    fn add(self, duration: Duration) -> Self {
        self.checked_add(duration).expect("timestamp overflow")
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self {
        self.checked_sub(duration).expect("timestamp overflow")
    }
}

impl Sub for Timestamp {
    type Output = Duration;

    fn sub(self, other: Self) -> Duration {
        // Preserve legacy elapsed-time behavior at UTC day boundaries.
        let seconds = self
            .unix_timestamp()
            .rem_euclid(SECONDS_PER_DAY)
            .cmp(&other.unix_timestamp().rem_euclid(SECONDS_PER_DAY));
        let extra = i64::from(self.leap_second && !seconds.is_lt())
            - i64::from(other.leap_second && !seconds.is_gt());
        self.local - other.local + Duration::seconds(extra)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_persisted_precision_and_offsets() {
        let instant = Timestamp::parse_rfc3339("2026-10-01t09:00:00.1234567890+09:00").unwrap();
        let utc = instant.to_utc();
        assert_eq!(instant, utc);
        assert_eq!(utc.to_rfc3339(), "2026-10-01T00:00:00.123456789+00:00");
        assert_eq!(
            utc.format_rfc3339(TimestampFormat::Micros, true),
            "2026-10-01T00:00:00.123456Z"
        );
        assert_eq!(
            utc.format_rfc3339(TimestampFormat::Millis, true),
            "2026-10-01T00:00:00.123Z"
        );
        let offset = Timestamp::parse_rfc3339("1969-12-31 20:59:59.999900−03:00").unwrap();
        assert_eq!(offset.unix_timestamp_millis(), -1);
        assert_eq!(offset.to_rfc3339(), "1969-12-31T20:59:59.999900-03:00");
        assert!(Timestamp::parse_rfc3339("2026-10-01X09:00:00Z").is_err());
        assert!(Timestamp::parse_rfc3339("2026-10-01T09:00:00+24:00").is_err());
    }

    #[test]
    fn preserves_the_leap_second_in_a_legacy_record() {
        let at = Timestamp::parse_rfc3339("2017-01-01T08:59:60.250+09:00").unwrap();
        let before = Timestamp::parse_rfc3339("2016-12-31T23:59:59.250Z").unwrap();
        let after = Timestamp::parse_rfc3339("2017-01-01T00:00:00.250Z").unwrap();
        assert_eq!(at.to_utc().to_rfc3339(), "2016-12-31T23:59:60.250+00:00");
        assert_eq!(at - before, Duration::SECOND);
        assert_eq!(after - at, Duration::ZERO);
        assert_eq!(at + Duration::SECOND, after);
        assert_eq!(at - Duration::SECOND, before);
    }
}
