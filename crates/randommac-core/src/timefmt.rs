//! UTC timestamps on top of the `time` crate.
//!
//! Every timestamp RandomMac keeps is an [`OffsetDateTime`] in UTC. In JSON it is RFC 3339 with
//! `Z` and only the fractional digits that are needed (`2026-11-18T01:02:03.456Z`), the same text
//! chrono wrote before 2.1, so files from older versions load unchanged and older versions can read
//! the new ones.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serializer};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{PrimitiveDateTime, UtcOffset};

pub use time::{Duration, OffsetDateTime};

/// The current time in UTC.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn utc(t: OffsetDateTime) -> OffsetDateTime {
    t.to_offset(UtcOffset::UTC)
}

/// RFC 3339 with `Z`, e.g. `2026-11-18T01:02:03.456789Z`.
pub fn to_rfc3339(t: OffsetDateTime) -> String {
    utc(t).format(&Rfc3339).unwrap_or_default()
}

/// RFC 3339 with exactly three fractional digits and `Z`, as in the log.
pub fn to_rfc3339_millis(t: OffsetDateTime) -> String {
    utc(t)
        .format(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
        ))
        .unwrap_or_default()
}

/// `YYYYMMDDhhmmssfff` — 17 digits, used in the names of quarantined files.
pub fn compact_millis(t: OffsetDateTime) -> String {
    utc(t)
        .format(format_description!(
            "[year][month][day][hour][minute][second][subsecond digits:3]"
        ))
        .unwrap_or_default()
}

/// `YYYYMMDD-hhmmss`, used in the names of exported files.
pub fn file_stamp(t: OffsetDateTime) -> String {
    utc(t)
        .format(format_description!(
            "[year][month][day]-[hour][minute][second]"
        ))
        .unwrap_or_default()
}

/// `YYYY-MM-DD hh:mm:ss` in UTC, for the command-line tool.
pub fn display(t: OffsetDateTime) -> String {
    utc(t)
        .format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .unwrap_or_default()
}

/// Parses RFC 3339 with any offset and converts it to UTC.
pub fn parse_rfc3339(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text.trim(), &Rfc3339).ok().map(utc)
}

/// Parses a date and time without an offset (`2024-05-10T12:34:56` with an optional fraction of any
/// length, as .NET writes it) and takes it as UTC.
pub fn parse_naive_as_utc(text: &str) -> Option<OffsetDateTime> {
    PrimitiveDateTime::parse(
        text.trim(),
        format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond]]]"
        ),
    )
    .ok()
    .map(PrimitiveDateTime::assume_utc)
}

/// Serde support for `#[serde(with = "timefmt::rfc3339")]`.
pub mod rfc3339 {
    use super::*;

    pub fn serialize<S: Serializer>(t: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&to_rfc3339(*t))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<OffsetDateTime, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_rfc3339(&text)
            .ok_or_else(|| D::Error::custom(format!("invalid RFC 3339 timestamp: {text:?}")))
    }

    /// The same for `Option<OffsetDateTime>` (`null` or missing = `None`).
    pub mod option {
        use super::*;

        pub fn serialize<S: Serializer>(
            t: &Option<OffsetDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match t {
                Some(t) => super::serialize(t, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<OffsetDateTime>, D::Error> {
            Option::<String>::deserialize(deserializer)?
                .map(|text| {
                    parse_rfc3339(&text).ok_or_else(|| {
                        D::Error::custom(format!("invalid RFC 3339 timestamp: {text:?}"))
                    })
                })
                .transpose()
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use time::macros::datetime;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Stamp {
        #[serde(with = "rfc3339")]
        at: OffsetDateTime,
        #[serde(
            default,
            with = "rfc3339::option",
            skip_serializing_if = "Option::is_none"
        )]
        maybe: Option<OffsetDateTime>,
    }

    #[test]
    fn writes_rfc3339_with_z_and_only_the_needed_digits() {
        assert_eq!(
            to_rfc3339(datetime!(2026-01-01 0:00 UTC)),
            "2026-01-01T00:00:00Z"
        );
        assert_eq!(
            to_rfc3339(datetime!(2026-01-01 0:00:00.5 UTC)),
            "2026-01-01T00:00:00.5Z"
        );
        assert_eq!(
            to_rfc3339(datetime!(2026-01-01 0:00:00.123456789 UTC)),
            "2026-01-01T00:00:00.123456789Z"
        );
        // Another offset is converted to UTC.
        assert_eq!(
            to_rfc3339(datetime!(2026-01-01 7:00 +07:00)),
            "2026-01-01T00:00:00Z"
        );
    }

    #[test]
    fn reads_what_chrono_wrote() {
        for (text, expected) in [
            ("2026-01-01T00:00:00Z", datetime!(2026-01-01 0:00 UTC)),
            (
                "2026-01-01T00:00:00.5Z",
                datetime!(2026-01-01 0:00:00.5 UTC),
            ),
            (
                "2026-01-01T00:00:00.123456789Z",
                datetime!(2026-01-01 0:00:00.123456789 UTC),
            ),
            ("2026-01-01T07:00:00+07:00", datetime!(2026-01-01 0:00 UTC)),
            ("2026-01-01T00:00:00+00:00", datetime!(2026-01-01 0:00 UTC)),
        ] {
            let parsed = parse_rfc3339(text).expect(text);
            assert_eq!(parsed, expected, "{text}");
            assert_eq!(parsed.offset(), UtcOffset::UTC, "{text}");
        }
        assert!(parse_rfc3339("2026-01-01 00:00:00").is_none());
        assert!(parse_rfc3339("yesterday").is_none());
    }

    #[test]
    fn fixed_formats() {
        let t = datetime!(2026-11-18 01:02:03.456789 UTC);
        assert_eq!(to_rfc3339_millis(t), "2026-11-18T01:02:03.456Z");
        assert_eq!(
            to_rfc3339_millis(datetime!(2026-11-18 01:02:03 UTC)),
            "2026-11-18T01:02:03.000Z"
        );
        assert_eq!(compact_millis(t), "20261118010203456");
        assert_eq!(compact_millis(t).len(), 17);
        assert_eq!(file_stamp(t), "20261118-010203");
        assert_eq!(display(t), "2026-11-18 01:02:03");
        assert_eq!(
            display(datetime!(2026-11-18 08:02:03 +07:00)),
            "2026-11-18 01:02:03"
        );
    }

    #[test]
    fn naive_timestamps_are_taken_as_utc() {
        assert_eq!(
            parse_naive_as_utc("2024-05-10T12:34:56"),
            Some(datetime!(2024-05-10 12:34:56 UTC))
        );
        assert_eq!(
            parse_naive_as_utc("2024-05-10T12:34:56.1234567"),
            Some(datetime!(2024-05-10 12:34:56.1234567 UTC))
        );
        assert!(parse_naive_as_utc("2024-05-10").is_none());
        assert!(parse_naive_as_utc("2024-05-10T12:34:56Z").is_none());
    }

    #[test]
    fn serde_round_trip() {
        let value = Stamp {
            at: datetime!(2026-01-01 0:00:00.5 UTC),
            maybe: None,
        };
        let json = serde_json::to_string(&value).expect("ser");
        assert_eq!(json, r#"{"at":"2026-01-01T00:00:00.5Z"}"#);
        assert_eq!(serde_json::from_str::<Stamp>(&json).expect("de"), value);

        let with = Stamp {
            at: datetime!(2026-01-01 0:00 UTC),
            maybe: Some(datetime!(2026-02-03 4:05:06 UTC)),
        };
        let json = serde_json::to_string(&with).expect("ser");
        assert_eq!(
            json,
            r#"{"at":"2026-01-01T00:00:00Z","maybe":"2026-02-03T04:05:06Z"}"#
        );
        assert_eq!(serde_json::from_str::<Stamp>(&json).expect("de"), with);

        let null: Stamp =
            serde_json::from_str(r#"{"at":"2026-01-01T00:00:00Z","maybe":null}"#).expect("null");
        assert_eq!(null.maybe, None);
        assert!(serde_json::from_str::<Stamp>(r#"{"at":"soon"}"#).is_err());
        assert!(
            serde_json::from_str::<Stamp>(r#"{"at":"2026-01-01T00:00:00Z","maybe":"x"}"#).is_err()
        );
    }

    #[test]
    fn now_is_utc() {
        assert_eq!(now().offset(), UtcOffset::UTC);
    }
}
