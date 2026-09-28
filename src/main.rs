use anyhow::{bail, Context, Result};
use chrono::{Datelike, LocalResult, NaiveDate, Offset, TimeZone};
use chrono_tz::Tz;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::str::FromStr;

#[derive(Parser)]
#[command(
    name = "chronogrid",
    version,
    about = "Temporal chaos and calendar conformance lab",
    long_about = "ChronoGrid tests and inspects difficult calendar, timezone, recurrence and date edge cases."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect dates and calendar properties.
    Date {
        #[command(subcommand)]
        command: DateCommands,
    },
}

#[derive(Subcommand)]
enum DateCommands {
    /// Inspect one Gregorian date in a specific IANA timezone.
    Inspect {
        /// Gregorian date in YYYY-MM-DD format.
        date: String,

        /// IANA timezone, for example America/New_York.
        #[arg(long, default_value = "UTC")]
        timezone: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Serialize)]
struct DateInspection {
    date: String,
    timezone: String,

    year: i32,
    month: u32,
    day: u32,

    weekday: String,
    ordinal_day: u32,

    iso_week_year: i32,
    iso_week: u32,
    iso_weekday: u32,

    quarter: u32,
    days_in_month: u32,
    days_in_year: u32,
    days_remaining_in_year: u32,

    is_leap_year: bool,

    midnight_status: String,
    utc_offset_seconds: Option<i32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Date { command } => match command {
            DateCommands::Inspect {
                date,
                timezone,
                format,
            } => inspect_date(&date, &timezone, format),
        },
    }
}

fn inspect_date(date_input: &str, timezone_input: &str, format: OutputFormat) -> Result<()> {
    let date = NaiveDate::parse_from_str(date_input, "%Y-%m-%d")
        .with_context(|| format!("invalid date '{date_input}'; expected YYYY-MM-DD"))?;

    let timezone = Tz::from_str(timezone_input)
        .map_err(|_| anyhow::anyhow!("unknown IANA timezone '{timezone_input}'"))?;

    let midnight = date
        .and_hms_opt(0, 0, 0)
        .context("could not construct local midnight")?;

    let (midnight_status, utc_offset_seconds) = match timezone.from_local_datetime(&midnight) {
        LocalResult::Single(datetime) => (
            "single".to_string(),
            Some(datetime.offset().fix().local_minus_utc()),
        ),

        LocalResult::Ambiguous(first, second) => (
            format!(
                "ambiguous: offsets {} and {} seconds",
                first.offset().fix().local_minus_utc(),
                second.offset().fix().local_minus_utc()
            ),
            None,
        ),

        LocalResult::None => ("nonexistent local time".to_string(), None),
    };

    let iso_week = date.iso_week();
    let leap_year = is_leap_year(date.year());
    let days_in_year = if leap_year { 366 } else { 365 };

    let inspection = DateInspection {
        date: date.format("%Y-%m-%d").to_string(),
        timezone: timezone.to_string(),

        year: date.year(),
        month: date.month(),
        day: date.day(),

        weekday: date.format("%A").to_string(),
        ordinal_day: date.ordinal(),

        iso_week_year: iso_week.year(),
        iso_week: iso_week.week(),
        iso_weekday: date.weekday().number_from_monday(),

        quarter: ((date.month() - 1) / 3) + 1,
        days_in_month: days_in_month(date.year(), date.month())?,
        days_in_year,
        days_remaining_in_year: days_in_year - date.ordinal(),

        is_leap_year: leap_year,

        midnight_status,
        utc_offset_seconds,
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&inspection)?);
        }

        OutputFormat::Text => {
            print_text_report(&inspection);
        }
    }

    Ok(())
}

fn print_text_report(inspection: &DateInspection) {
    println!("ChronoGrid Date Inspection");
    println!("==========================");
    println!();

    println!("Civil date");
    println!("----------");
    println!("Date:               {}", inspection.date);
    println!("Timezone:           {}", inspection.timezone);
    println!("Weekday:            {}", inspection.weekday);
    println!("Year:               {}", inspection.year);
    println!("Month:              {}", inspection.month);
    println!("Day:                {}", inspection.day);
    println!();

    println!("Calendar position");
    println!("-----------------");
    println!("Quarter:            Q{}", inspection.quarter);
    println!("Ordinal day:        {:03}", inspection.ordinal_day);
    println!("Days in month:      {}", inspection.days_in_month);
    println!("Days in year:       {}", inspection.days_in_year);
    println!("Days remaining:     {}", inspection.days_remaining_in_year);
    println!("Leap year:          {}", inspection.is_leap_year);
    println!();

    println!("ISO week date");
    println!("-------------");
    println!(
        "ISO representation: {}-W{:02}-{}",
        inspection.iso_week_year, inspection.iso_week, inspection.iso_weekday
    );
    println!("ISO week-year:      {}", inspection.iso_week_year);
    println!("ISO week:           {}", inspection.iso_week);
    println!("ISO weekday:        {}", inspection.iso_weekday);
    println!();

    println!("Timezone analysis");
    println!("-----------------");
    println!("Midnight status:    {}", inspection.midnight_status);

    match inspection.utc_offset_seconds {
        Some(seconds) => {
            println!("UTC offset seconds: {}", seconds);
            println!("UTC offset:         {}", format_utc_offset(seconds));
        }
        None => {
            println!("UTC offset:         context-dependent");
        }
    }

    println!();
    println!("ChronoGrid");
    println!("Temporal chaos and calendar conformance lab");
    println!("Developed by JW Calendar");
    println!("https://jwcalendar.com/");
}

fn format_utc_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let absolute = seconds.abs();

    let hours = absolute / 3600;
    let minutes = (absolute % 3600) / 60;

    format!("{sign}{hours:02}:{minutes:02}")
}

fn days_in_month(year: i32, month: u32) -> Result<u32> {
    if !(1..=12).contains(&month) {
        bail!("month must be between 1 and 12");
    }

    let first_of_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .context("date is outside supported chrono range")?;

    let last_day = first_of_next_month
        .pred_opt()
        .context("date is outside supported chrono range")?;

    Ok(last_day.day())
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_year_rules_are_correct() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2027));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2400));
    }

    #[test]
    fn february_has_28_days_in_2027() {
        assert_eq!(days_in_month(2027, 2).unwrap(), 28);
    }

    #[test]
    fn february_has_29_days_in_leap_year() {
        assert_eq!(days_in_month(2028, 2).unwrap(), 29);
    }

    #[test]
    fn december_has_31_days() {
        assert_eq!(days_in_month(2027, 12).unwrap(), 31);
    }

    #[test]
    fn utc_offset_formatting_is_correct() {
        assert_eq!(format_utc_offset(0), "+00:00");
        assert_eq!(format_utc_offset(10_800), "+03:00");
        assert_eq!(format_utc_offset(-18_000), "-05:00");
        assert_eq!(format_utc_offset(19_800), "+05:30");
    }

    #[test]
    fn january_first_2027_has_correct_calendar_properties() {
        let date = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();

        assert_eq!(date.weekday().to_string(), "Fri");
        assert_eq!(date.ordinal(), 1);
        assert_eq!(date.iso_week().year(), 2026);
        assert_eq!(date.iso_week().week(), 53);
    }
}
