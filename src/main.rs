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
    /// Inspect a civil date and its calendar properties.
    Date {
        #[command(subcommand)]
        command: DateCommands,
    },
}

#[derive(Subcommand)]
enum DateCommands {
    /// Inspect one date in a specific IANA timezone.
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
        LocalResult::Single(dt) => (
            "single".to_string(),
            Some(dt.offset().fix().local_minus_utc()),
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

    let iso = date.iso_week();

    let inspection = DateInspection {
        date: date.format("%Y-%m-%d").to_string(),
        timezone: timezone.name().to_string(),
        year: date.year(),
        month: date.month(),
        day: date.day(),
        weekday: date.format("%A").to_string(),
        ordinal_day: date.ordinal(),
        iso_week_year: iso.year(),
        iso_week: iso.week(),
        iso_weekday: date.weekday().number_from_monday(),
        quarter: ((date.month() - 1) / 3) + 1,
        days_in_month: days_in_month(date.year(), date.month())?,
        is_leap_year: is_leap_year(date.year()),
        midnight_status,
        utc_offset_seconds,
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&inspection)?);
        }
        OutputFormat::Text => {
            println!("ChronoGrid Date Inspection");
            println!("==========================");
            println!("Date:              {}", inspection.date);
            println!("Timezone:          {}", inspection.timezone);
            println!("Weekday:           {}", inspection.weekday);
            println!("Ordinal day:       {:03}", inspection.ordinal_day);
            println!(
                "ISO week date:      {}-W{:02}-{}",
                inspection.iso_week_year,
                inspection.iso_week,
                inspection.iso_weekday
            );
            println!("Quarter:           Q{}", inspection.quarter);
            println!("Days in month:     {}", inspection.days_in_month);
            println!("Leap year:         {}", inspection.is_leap_year);
            println!("Midnight status:   {}", inspection.midnight_status);

            match inspection.utc_offset_seconds {
                Some(seconds) => println!("UTC offset:        {} seconds", seconds),
                None => println!("UTC offset:        context-dependent"),
            }

            println!();
            println!("Project: https://jwcalendar.com/");
        }
    }

    Ok(())
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
