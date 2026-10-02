use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use std::str::FromStr;

use crate::legacy::OutputFormat;

#[derive(Serialize)]
struct Candidate {
    utc: String,
    offset: String,
    offset_seconds: i32,
}
#[derive(Serialize)]
struct Classification {
    schema_version: u8,
    chronogrid_version: &'static str,
    local_datetime: String,
    timezone: String,
    classification: &'static str,
    candidates: Vec<Candidate>,
    transition: Option<String>,
}
#[derive(Serialize)]
struct MatrixRow {
    timezone: String,
    local_datetime: String,
    utc: String,
    utc_offset: String,
    utc_offset_seconds: i32,
    date_shift_days: i64,
    date_rollover: String,
}
#[derive(Serialize)]
struct Matrix {
    schema_version: u8,
    chronogrid_version: &'static str,
    source_local_datetime: String,
    source_timezone: String,
    utc: String,
    rows: Vec<MatrixRow>,
}

fn timezone(name: &str) -> Result<Tz> {
    Tz::from_str(name).map_err(|_| anyhow::anyhow!("unknown IANA timezone '{name}'"))
}
pub(crate) fn parse_local(input: &str) -> Result<NaiveDateTime> {
    [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ]
    .iter()
    .find_map(|fmt| NaiveDateTime::parse_from_str(input, fmt).ok())
    .with_context(|| format!("invalid local date-time '{input}'; use YYYY-MM-DDTHH:MM[:SS]"))
}
fn offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let n = seconds.abs();
    format!("{sign}{:02}:{:02}", n / 3600, (n % 3600) / 60)
}
fn candidate<T: TimeZone>(dt: DateTime<T>) -> Candidate
where
    T::Offset: Offset,
{
    let secs = dt.offset().fix().local_minus_utc();
    Candidate {
        utc: dt.with_timezone(&Utc).to_rfc3339(),
        offset: offset(secs),
        offset_seconds: secs,
    }
}
fn nearest_valid(tz: Tz, local: NaiveDateTime, dir: i64) -> Option<(NaiveDateTime, i32)> {
    for minutes in 1..=24 * 60 {
        let Some(probe) = local.checked_add_signed(Duration::minutes(dir * minutes)) else {
            break;
        };
        if let LocalResult::Single(dt) = tz.from_local_datetime(&probe) {
            return Some((probe, dt.offset().fix().local_minus_utc()));
        }
    }
    None
}
fn classify_data(local: NaiveDateTime, zone: &str) -> Result<Classification> {
    let tz = timezone(zone)?;
    let (classification, candidates, transition) = match tz.from_local_datetime(&local) {
        LocalResult::Single(a) => ("unique", vec![candidate(a)], None),
        LocalResult::Ambiguous(a, b) => {
            let mut c = vec![candidate(a), candidate(b)];
            c.sort_by(|x, y| x.utc.cmp(&y.utc));
            ("ambiguous", c, None)
        }
        LocalResult::None => {
            let before = nearest_valid(tz, local, -1);
            let after = nearest_valid(tz, local, 1);
            let detail = match (before, after) {
                (Some((b, bo)), Some((a, ao))) => Some(format!("local clock jumps forward from UTC{} before {} to UTC{} after {}; the requested time falls in the gap", offset(bo), b, offset(ao), a)),
                _ => Some("no valid local time was found within 24 hours on either side".into()),
            };
            ("nonexistent", Vec::new(), detail)
        }
    };
    Ok(Classification {
        schema_version: 1,
        chronogrid_version: env!("CARGO_PKG_VERSION"),
        local_datetime: local.to_string(),
        timezone: tz.to_string(),
        classification,
        candidates,
        transition,
    })
}
pub(crate) fn classify(input: &str, zone: &str, format: OutputFormat) -> Result<()> {
    let data = classify_data(parse_local(input)?, zone)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&data)?),
        OutputFormat::Text => {
            println!(
                "{} in {}: {}",
                data.local_datetime, data.timezone, data.classification
            );
            for c in &data.candidates {
                println!("  UTC {}  offset {}", c.utc, c.offset);
            }
            if let Some(detail) = &data.transition {
                println!("  {detail}");
            }
        }
    }
    Ok(())
}
pub(crate) fn matrix(
    input: &str,
    source_zone: &str,
    targets: &str,
    later: Option<bool>,
    format: OutputFormat,
) -> Result<()> {
    let local = parse_local(input)?;
    let source_tz = timezone(source_zone)?;
    let instant = match source_tz.from_local_datetime(&local) {
        LocalResult::Single(dt) => dt.with_timezone(&Utc),
        LocalResult::Ambiguous(a, b) => match later {
            Some(true) => a.max(b).with_timezone(&Utc),
            Some(false) => a.min(b).with_timezone(&Utc),
            None => bail!("source time is ambiguous; specify --fold earlier or --fold later"),
        },
        LocalResult::None => {
            bail!("source local time does not exist in {source_zone}; refusing to normalize it")
        }
    };
    let mut rows = Vec::new();
    for name in targets.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let tz = timezone(name)?;
        let target = instant.with_timezone(&tz);
        let secs = target.offset().fix().local_minus_utc();
        let shift = (target.date_naive() - local.date()).num_days();
        rows.push(MatrixRow {
            timezone: tz.to_string(),
            local_datetime: target.naive_local().to_string(),
            utc: instant.to_rfc3339(),
            utc_offset: offset(secs),
            utc_offset_seconds: secs,
            date_shift_days: shift,
            date_rollover: match shift {
                0 => "same date".into(),
                -1 => "previous date".into(),
                1 => "next date".into(),
                n if n < 0 => format!("{n} days earlier"),
                n => format!("{n} days later"),
            },
        });
    }
    if rows.is_empty() {
        bail!("--to must contain at least one timezone");
    }
    let report = Matrix {
        schema_version: 1,
        chronogrid_version: env!("CARGO_PKG_VERSION"),
        source_local_datetime: local.to_string(),
        source_timezone: source_tz.to_string(),
        utc: instant.to_rfc3339(),
        rows,
    };
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Text => {
            println!(
                "{} {} = {}",
                report.source_local_datetime, report.source_timezone, report.utc
            );
            println!(
                "{:<28} {:<26} {:<9} {:>8}  Rollover",
                "Timezone", "Local time", "Offset", "Days"
            );
            for row in &report.rows {
                println!(
                    "{:<28} {:<26} {:<9} {:>8}  {}",
                    row.timezone,
                    row.local_datetime,
                    row.utc_offset,
                    row.date_shift_days,
                    row.date_rollover
                );
            }
        }
    }
    Ok(())
}
#[derive(Serialize)]
struct Doctor {
    schema_version: u8,
    chronogrid_version: &'static str,
    operating_system: &'static str,
    architecture: &'static str,
    timezone_engine: &'static str,
    timezone_database_version: Option<&'static str>,
    default_output: &'static str,
    deterministic_runtime: bool,
}
pub(crate) fn doctor(format: OutputFormat) -> Result<()> {
    let d = Doctor {
        schema_version: 1,
        chronogrid_version: env!("CARGO_PKG_VERSION"),
        operating_system: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        timezone_engine: "chrono-tz embedded IANA timezone data",
        timezone_database_version: None,
        default_output: "text",
        deterministic_runtime: true,
    };
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&d)?),
        OutputFormat::Text => {
            println!("ChronoGrid {}", d.chronogrid_version);
            println!("Platform: {} / {}", d.operating_system, d.architecture);
            println!("Timezone engine: {}", d.timezone_engine);
            println!("Timezone database version: not exposed by the dependency");
            println!(
                "Default output: {}; deterministic calculations: yes",
                d.default_output
            );
        }
    }
    Ok(())
}
#[allow(dead_code)]
pub(crate) fn parse_date(input: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .with_context(|| format!("invalid date '{input}'; expected YYYY-MM-DD"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classify_new_york_gap_fold_and_normal_time() {
        let gap = classify_data(
            parse_local("2027-03-14T02:30:00").unwrap(),
            "America/New_York",
        )
        .unwrap();
        assert_eq!(gap.classification, "nonexistent");
        assert!(gap.transition.as_deref().unwrap().contains("jumps forward"));
        let fold = classify_data(
            parse_local("2027-11-07T01:30:00").unwrap(),
            "America/New_York",
        )
        .unwrap();
        assert_eq!(fold.classification, "ambiguous");
        assert_eq!(fold.candidates.len(), 2);
        assert_ne!(fold.candidates[0].utc, fold.candidates[1].utc);
        let normal = classify_data(
            parse_local("2027-02-01T12:00:00").unwrap(),
            "America/New_York",
        )
        .unwrap();
        assert_eq!(normal.classification, "unique");
        assert_eq!(normal.candidates.len(), 1);
    }
    #[test]
    fn rejects_unknown_timezone() {
        assert!(
            classify_data(parse_local("2027-01-01T00:00:00").unwrap(), "Mars/Olympus").is_err()
        );
    }
}
