// DO NOT MODIFY THIS FILE!
// It was extracted nearly wholesale from eu4save. Maybe that means I need to
// move it to the common parser module

use jomini::Scalar;
use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;

const DAYS_PER_MONTH: [u8; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Struct specialized to parsing, formatting, and manipulating dates in Imperator
///
/// A date in Imperator does not follow any traditional calendar and instead views the
/// world on simpler terms: that every year should be treated as a non-leap year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImperatorDate {
    year: u16,
    month: u8,
    day: u8,
}

impl PartialOrd for ImperatorDate {
    fn partial_cmp(&self, other: &ImperatorDate) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ImperatorDate {
    fn cmp(&self, other: &ImperatorDate) -> Ordering {
        self.year
            .cmp(&other.year)
            .then_with(|| self.month.cmp(&other.month))
            .then_with(|| self.day.cmp(&other.day))
    }
}

impl ImperatorDate {
    /// Create a new Imperator date from year, month, and day parts
    ///
    /// Will return `None` if the date does not exist
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// assert_eq!(ImperatorDate::new(1444, 11, 11), ImperatorDate::parse_from_str("1444.11.11"));
    /// assert_eq!(ImperatorDate::new(800, 5, 3), ImperatorDate::parse_from_str("800.5.3"));
    /// assert!(ImperatorDate::new(800, 0, 3).is_none());
    /// assert!(ImperatorDate::new(800, 1, 0).is_none());
    /// assert!(ImperatorDate::new(800, 13, 1).is_none());
    /// assert!(ImperatorDate::new(800, 12, 32).is_none());
    /// assert!(ImperatorDate::new(2020, 2, 29).is_none());
    /// ```
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        if year != 0 && month != 0 && day != 0 {
            if let Some(&days) = DAYS_PER_MONTH.get(usize::from(month)) {
                if day <= days {
                    return Some(ImperatorDate { year, month, day });
                }
            }
        }

        None
    }

    /// Year of the date
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1445.02.03").expect("to parse date");
    /// assert_eq!(date.year(), 1445);
    /// ```
    pub fn year(&self) -> u16 {
        self.year
    }

    /// Month of the date
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1445.02.03").expect("to parse date");
    /// assert_eq!(date.month(), 2);
    /// ```
    pub fn month(&self) -> u8 {
        self.month
    }

    /// Day of the date
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1445.02.03").expect("to parse date");
    /// assert_eq!(date.day(), 3);
    /// ```
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Parses a string and returns a new ImperatorDate if valid.
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1444.11.11").expect("to parse date");
    /// assert_eq!(date.year(), 1444);
    /// assert_eq!(date.month(), 11);
    /// assert_eq!(date.day(), 11);
    /// ```
    pub fn parse_from_str<T: AsRef<str>>(s: T) -> Option<Self> {
        let data = s.as_ref().as_bytes();
        let mut state = 0;
        let mut span1: &[u8] = &[];
        let mut span2: &[u8] = &[];
        let mut start = 0;

        // micro-optimization: check the first byte to see if the first character (if available)
        // is outside our upper bound (ie: not a number). This micro optimization doesn't
        // harm the happy path (input is a date) by more than a few percent, but if the input
        // is not a date, this shaves off 20-25% in date parsing benchmarks.
        if data.get(0).map_or(true, |c| *c > b'9') {
            return None;
        }

        for (pos, &c) in data.iter().enumerate() {
            if c == b'.' {
                match state {
                    0 => {
                        span1 = &data[start..pos];
                        state = 1;
                    }
                    1 => {
                        span2 = &data[start..pos];
                        state = 2;
                    }
                    _ => return None,
                }
                start = pos + 1;
            } else if c > b'9' || c < b'0' {
                return None;
            }
        }

        let span3 = &data[start..];

        if let Ok(y) = Scalar::new(span1).to_u64() {
            if let Ok(m) = Scalar::new(span2).to_u64() {
                if let Ok(d) = Scalar::new(span3).to_u64() {
                    return ImperatorDate::new(y as u16, m as u8, d as u8);
                }
            }
        }

        None
    }

    pub fn days(&self) -> i32 {
        let mut days: i32 = 0;
        days += i32::from(self.year) * 365;
        days += match self.month {
            1 => -1,
            2 => 30,
            3 => 58,
            4 => 89,
            5 => 119,
            6 => 150,
            7 => 180,
            8 => 211,
            9 => 242,
            10 => 272,
            11 => 303,
            12 => 333,
            _ => unreachable!(),
        };
        days += i32::from(self.day);

        days
    }

    pub fn days_until(&self, other: &ImperatorDate) -> i32 {
        other.days() - self.days()
    }

    pub fn add_days(&self, days: i32) -> ImperatorDate {
        let new_days = self.days() + days;
        let days_since_jan1 = new_days % 365;
        let year = new_days / 365;
        let (month, day) = month_day_from_julian(days_since_jan1);

        ImperatorDate {
            year: year as u16,
            month: month as u8,
            day: day as u8,
        }
    }

    pub fn from_i32(mut s: i32) -> Option<Self> {
        let _hours = s % 24;
        s /= 24;
        let days_since_jan1 = s % 365;
        s /= 365;
        let year = s.checked_sub(5000).unwrap_or(0);
        if year < 1 {
            return None;
        }

        let (month, day) = month_day_from_julian(days_since_jan1);

        Some(ImperatorDate {
            year: year as u16,
            month: month as u8,
            day: day as u8,
        })
    }

    /// Formats an Imperator date in the ISO 8601 format: YYYY-MM-DD
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1400.1.2").expect("to parse date");
    /// assert_eq!(date.iso_8601(), String::from("1400-01-02"));
    /// ```
    pub fn iso_8601(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Formats an Imperator date in the Imperator format: Y.M.D
    ///
    /// ```
    /// use imperator_save::ImperatorDate;
    /// let date = ImperatorDate::parse_from_str("1400.1.2").expect("to parse date");
    /// let end_date = date.add_days(30);
    /// assert_eq!(end_date.imperator_fmt(), String::from("1400.2.1"));
    /// ```
    pub fn imperator_fmt(&self) -> String {
        format!("{}.{}.{}", self.year, self.month, self.day)
    }
}

fn month_day_from_julian(days_since_jan1: i32) -> (i32, i32) {
    // https://landweb.modaps.eosdis.nasa.gov/browse/calendar.html
    // except we start at 0 instead of 1
    match days_since_jan1 {
        0..=30 => (1, days_since_jan1 + 1),
        31..=58 => (2, days_since_jan1 - 30),
        59..=89 => (3, days_since_jan1 - 58),
        90..=119 => (4, days_since_jan1 - 89),
        120..=150 => (5, days_since_jan1 - 119),
        151..=180 => (6, days_since_jan1 - 150),
        181..=211 => (7, days_since_jan1 - 180),
        212..=242 => (8, days_since_jan1 - 211),
        243..=272 => (9, days_since_jan1 - 242),
        273..=303 => (10, days_since_jan1 - 272),
        304..=333 => (11, days_since_jan1 - 303),
        334..=364 => (12, days_since_jan1 - 333),
        _ => unreachable!(),
    }
}

impl Serialize for ImperatorDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.iso_8601().as_str())
    }
}

struct ImperatorDateVisitor;

impl<'de> Visitor<'de> for ImperatorDateVisitor {
    type Value = ImperatorDate;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an imperator date")
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ImperatorDate::from_i32(v)
            .ok_or_else(|| de::Error::custom(format!("could not convert {} to a date", v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ImperatorDate::parse_from_str(v)
            .ok_or_else(|| de::Error::custom(format!("invalid date: {}", v)))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(v.as_str())
    }
}

impl<'de> Deserialize<'de> for ImperatorDate {
    fn deserialize<D>(deserializer: D) -> Result<ImperatorDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ImperatorDateVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_roundtrip() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        assert_eq!(date.iso_8601(), String::from("1400-01-02"));
    }

    #[test]
    fn test_imperator_fmt() {
        let test_cases = [
            "1400.1.2",
            "1457.3.5",
            "1.1.1",
            "1444.11.11",
            "1444.11.30",
            "1444.2.19",
        ];

        for case in &test_cases {
            let date = ImperatorDate::parse_from_str(case).unwrap();
            assert_eq!(date.imperator_fmt(), case.to_string());
        }
    }

    #[test]
    fn test_first_bin_date() {
        let date = ImperatorDate::from_i32(56379360).unwrap();
        assert_eq!(date.iso_8601(), String::from("1436-01-01"));
    }

    #[test]
    fn test_november_date_regression() {
        let date = ImperatorDate::from_i32(56379360).unwrap().add_days(303);
        assert_eq!(date.iso_8601(), String::from("1436-10-31"));
        let date = ImperatorDate::from_i32(56379360).unwrap().add_days(304);
        assert_eq!(date.iso_8601(), String::from("1436-11-01"));
        let date = ImperatorDate::from_i32(56379360)
            .unwrap()
            .add_days(303 - 30);
        assert_eq!(date.iso_8601(), String::from("1436-10-01"));
        let date = ImperatorDate::from_i32(56379360)
            .unwrap()
            .add_days(303 - 31);
        assert_eq!(date.iso_8601(), String::from("1436-09-30"));
        let date = ImperatorDate::from_i32(56379360)
            .unwrap()
            .add_days(303 - 31 - 29);
        assert_eq!(date.iso_8601(), String::from("1436-09-01"));
        let date = ImperatorDate::from_i32(56379360)
            .unwrap()
            .add_days(303 - 31 - 30);
        assert_eq!(date.iso_8601(), String::from("1436-08-31"));
    }

    #[test]
    fn test_past_leap_year_bin_date() {
        let date = ImperatorDate::from_i32(59611248).unwrap();
        assert_eq!(date.iso_8601(), String::from("1804-12-09"));
    }

    #[test]
    fn test_early_leap_year_bin_date() {
        let date = ImperatorDate::from_i32(57781584).unwrap();
        assert_eq!(date.iso_8601(), String::from("1596-01-27"));
    }

    #[test]
    fn test_non_leap_year_bin_date() {
        let date = ImperatorDate::from_i32(57775944).unwrap();
        assert_eq!(date.iso_8601(), String::from("1595-06-06"));
    }

    #[test]
    fn test_early_date() {
        let date = ImperatorDate::from_i32(43808760).unwrap();
        assert_eq!(date.iso_8601(), String::from("0001-01-01"));
    }

    #[test]
    fn test_days_until() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let date2 = ImperatorDate::parse_from_str("1400.1.3").unwrap();
        assert_eq!(1, date.days_until(&date2));
    }

    #[test]
    fn test_days_until2() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let date2 = ImperatorDate::parse_from_str("1401.1.2").unwrap();
        assert_eq!(365, date.days_until(&date2));
    }

    #[test]
    fn test_days_until3() {
        let date = ImperatorDate::parse_from_str("1400.1.1").unwrap();
        let date2 = ImperatorDate::parse_from_str("1401.12.31").unwrap();
        assert_eq!(729, date.days_until(&date2));
    }

    #[test]
    fn test_days_until4() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let date2 = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        assert_eq!(0, date.days_until(&date2));
    }

    #[test]
    fn test_days_until5() {
        let date = ImperatorDate::parse_from_str("1400.1.1").unwrap();
        let date2 = ImperatorDate::parse_from_str("1401.12.31").unwrap();
        assert_eq!(-729, date2.days_until(&date));
    }

    #[test]
    fn test_add_days() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let actual = date.add_days(1);
        let expected = ImperatorDate::parse_from_str("1400.1.3").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_days2() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let actual = date.add_days(365);
        let expected = ImperatorDate::parse_from_str("1401.1.2").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_days3() {
        let date = ImperatorDate::parse_from_str("1400.1.1").unwrap();
        let actual = date.add_days(729);
        let expected = ImperatorDate::parse_from_str("1401.12.31").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_days4() {
        let date = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        let actual = date.add_days(0);
        let expected = ImperatorDate::parse_from_str("1400.1.2").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_all_days() {
        let start = ImperatorDate::parse_from_str("1400.1.1").unwrap();
        for i in 0..364 {
            let (month, day) = month_day_from_julian(i);
            let next = ImperatorDate::parse_from_str(format!("1400.{}.{}", month, day)).unwrap();
            assert_eq!(start.add_days(i), next);
            assert_eq!(start.days_until(&next), i);
        }
    }

    #[test]
    fn test_cmp() {
        let date = ImperatorDate::parse_from_str("1457.3.5").unwrap();
        let date2 = ImperatorDate::parse_from_str("1457.3.4").unwrap();
        assert!(date2 < date);
    }
}
