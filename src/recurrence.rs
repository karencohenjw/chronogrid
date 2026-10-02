use anyhow::{bail, Context, Result};
use chrono::{LocalResult, TimeZone};
use chrono_tz::Tz;
use rrule::RRuleSet;
use serde::Serialize;
use std::str::FromStr;

use crate::{legacy::OutputFormat, temporal};

#[derive(Serialize)]
struct Expansion {
    schema_version: u8,
    chronogrid_version: &'static str,
    start: String,
    timezone: String,
    rule: String,
    limit: u32,
    truncated: bool,
    occurrences: Vec<String>,
}

pub(crate) fn expand(
    start: &str,
    zone: &str,
    rule: &str,
    limit: u32,
    format: OutputFormat,
) -> Result<()> {
    if limit == 0 || limit > 10_000 {
        bail!("--limit must be between 1 and 10000");
    }
    let local = temporal::parse_local(start)?;
    let tz = Tz::from_str(zone).map_err(|_| anyhow::anyhow!("unknown IANA timezone '{zone}'"))?;
    let start_value = match tz.from_local_datetime(&local) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(_, _) => {
            bail!("DTSTART is ambiguous in {zone}; choose an unambiguous wall time")
        }
        LocalResult::None => bail!("DTSTART does not exist in {zone}"),
    };
    let dtstart = if zone == "UTC" || zone == "Etc/UTC" || zone == "GMT" {
        format!("DTSTART:{}Z", local.format("%Y%m%dT%H%M%S"))
    } else {
        format!("DTSTART;TZID={}:{}", tz, local.format("%Y%m%dT%H%M%S"))
    };
    let input = format!("{dtstart}\nRRULE:{rule}");
    let set = RRuleSet::from_str(&input)
        .with_context(|| format!("invalid or unsupported RRULE '{rule}'"))?;
    let result = set.all((limit + 1) as u16);
    let truncated = result.dates.len() > limit as usize;
    let occurrences = result
        .dates
        .into_iter()
        .take(limit as usize)
        .map(|d| d.to_rfc3339())
        .collect();
    let report = Expansion {
        schema_version: 1,
        chronogrid_version: env!("CARGO_PKG_VERSION"),
        start: start_value.to_rfc3339(),
        timezone: tz.to_string(),
        rule: rule.into(),
        limit,
        truncated,
        occurrences,
    };
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Text => {
            println!("RRULE expansion ({}; {})", report.timezone, report.rule);
            for (index, value) in report.occurrences.iter().enumerate() {
                println!("{:>4}  {value}", index + 1);
            }
            if report.truncated {
                println!(
                    "Truncated at the configured limit of {} occurrences.",
                    report.limit
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    #[test]
    fn monthly_first_monday_count_is_honored() {
        let input = "DTSTART;TZID=Europe/Istanbul:20270104T090000\nRRULE:FREQ=MONTHLY;BYDAY=MO;BYSETPOS=1;COUNT=12";
        let set = RRuleSet::from_str(input).unwrap();
        let dates = set.all(13).dates;
        assert_eq!(dates.len(), 12);
        assert!(dates.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(dates.iter().all(|d| d.time().hour() == 9));
        assert!(dates.iter().all(|d| d.weekday() == chrono::Weekday::Mon));
    }
    #[test]
    fn invalid_rule_is_rejected() {
        let input = "DTSTART:20270101T090000Z\nRRULE:FREQ=FORTNIGHTLY;COUNT=2";
        assert!(RRuleSet::from_str(input).is_err());
    }
}
