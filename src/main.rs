use anyhow::{bail, Context, Result};
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone,
    Timelike, Utc,
};
use chrono_tz::Tz;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[command(
    name = "chronogrid",
    version,
    about = "Can your software survive time?",
    long_about = "A deterministic temporal and calendar conformance laboratory."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Date {
        #[command(subcommand)]
        command: DateCommand,
    },
    Week {
        #[command(subcommand)]
        command: WeekCommand,
    },
    Calendar {
        #[command(subcommand)]
        command: CalendarCommand,
    },
    Convert {
        system: String,
        date: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
    Timezone {
        #[command(subcommand)]
        command: TimezoneCommand,
    },
    Challenge {
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    Chaos {
        #[arg(long = "from", default_value_t = 1900)]
        from_year: i32,
        #[arg(long = "to", default_value_t = 2100)]
        to_year: i32,
        #[arg(long, default_value = "chronogrid-demo")]
        seed: String,
        #[arg(long, default_value_t = 25)]
        count: usize,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum DateCommand {
    Inspect {
        date: String,
        #[arg(long, default_value = "UTC")]
        timezone: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum WeekCommand {
    Inspect {
        date: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum CalendarCommand {
    Month {
        year: i32,
        month: u32,
        #[arg(long, value_enum, default_value_t=WeekStart::Monday)]
        week_start: WeekStart,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum TimezoneCommand {
    Inspect {
        zone: String,
        #[arg(long)]
        year: i32,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
#[derive(Copy, Clone, Debug, ValueEnum)]
enum WeekStart {
    Monday,
    Sunday,
}
impl WeekStart {
    fn number(self) -> u32 {
        match self {
            Self::Monday => 1,
            Self::Sunday => 7,
        }
    }
}
#[derive(Debug, Serialize)]
struct DateInspection {
    gregorian_date: String,
    timezone: String,
    year: i32,
    month: u32,
    day: u32,
    weekday: String,
    quarter: u32,
    ordinal_day: u32,
    days_in_month: u32,
    days_in_year: u32,
    days_remaining: u32,
    is_leap_year: bool,
    iso_week_year: i32,
    iso_week: u32,
    iso_weekday: u32,
    local_midnight_status: String,
    utc_offset_seconds: Option<i32>,
    utc_offset: Option<String>,
}
#[derive(Debug, Serialize)]
struct WeekInspection {
    gregorian_date: String,
    iso_week_year: i32,
    iso_week: u32,
    iso_weekday: u32,
}
#[derive(Debug, Serialize)]
struct CalendarDay {
    date: String,
    day: u32,
    in_month: bool,
}
#[derive(Debug, Serialize)]
struct CalendarMonth {
    year: i32,
    month: u32,
    week_start: String,
    weekdays: Vec<String>,
    weeks: Vec<Vec<CalendarDay>>,
}
#[derive(Debug, Serialize)]
struct Conversion {
    gregorian_date: String,
    ordinal_date: String,
    iso_week_date: String,
    astronomical_julian_date_at_utc_midnight: f64,
    julian_day_number_at_utc_noon: i64,
    modified_julian_date_at_utc_midnight: f64,
    terminology_note: String,
}
#[derive(Debug, Serialize)]
struct Transition {
    utc: String,
    local: String,
    old_offset_seconds: i32,
    new_offset_seconds: i32,
    old_offset: String,
    new_offset: String,
    change: String,
}
#[derive(Debug, Serialize)]
struct TransitionReport {
    timezone: String,
    year: i32,
    transitions: Vec<Transition>,
}
#[derive(Debug, Serialize)]
struct ChallengeVector {
    id: String,
    class: String,
    passed: bool,
}
#[derive(Debug, Serialize)]
struct ChallengeReport {
    version: String,
    vectors_executed: usize,
    passed: usize,
    failed: usize,
    vectors: Vec<ChallengeVector>,
    reproduction_command: String,
}
#[derive(Debug, Serialize)]
struct ChaosVector {
    id: String,
    class: String,
    date: String,
    timezone: String,
    note: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Date {
            command:
                DateCommand::Inspect {
                    date,
                    timezone,
                    format,
                },
        } => date_inspect(&date, &timezone, format),
        Commands::Week {
            command: WeekCommand::Inspect { date, format },
        } => week_inspect(&date, format),
        Commands::Calendar {
            command:
                CalendarCommand::Month {
                    year,
                    month,
                    week_start,
                    format,
                },
        } => calendar_month(year, month, week_start, format),
        Commands::Convert {
            system,
            date,
            format,
        } => {
            if system.to_lowercase() != "gregorian" {
                bail!("only the Gregorian civil calendar is supported by convert");
            }
            convert_gregorian(&date, format)
        }
        Commands::Timezone {
            command: TimezoneCommand::Inspect { zone, year, format },
        } => timezone_inspect(&zone, year, format),
        Commands::Challenge { format, report } => challenge(format, report),
        Commands::Chaos {
            from_year,
            to_year,
            seed,
            count,
            format,
        } => chaos(from_year, to_year, &seed, count, format),
    }
}
fn parse_date(input: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .with_context(|| format!("invalid Gregorian date '{input}'; expected YYYY-MM-DD"))
}
fn parse_tz(input: &str) -> Result<Tz> {
    Tz::from_str(input).map_err(|_| anyhow::anyhow!("unknown IANA timezone '{input}'"))
}
fn is_leap_year(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
fn days_in_month(y: i32, m: u32) -> Result<u32> {
    if !(1..=12).contains(&m) {
        bail!("month must be between 1 and 12");
    }
    let next = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
    }
    .context("year is outside the supported date range")?;
    Ok(next
        .pred_opt()
        .context("date outside supported range")?
        .day())
}
fn offset_string(s: i32) -> String {
    let sign = if s < 0 { '-' } else { '+' };
    let a = s.abs();
    format!("{sign}{:02}:{:02}", a / 3600, (a % 3600) / 60)
}
fn emit<T: Serialize>(value: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
        OutputFormat::Text => println!("{}", serde_json::to_string_pretty(value)?),
    }
    Ok(())
}
fn date_inspect(input: &str, zone_input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let tz = parse_tz(zone_input)?;
    let naive = d.and_hms_opt(0, 0, 0).context("invalid local midnight")?;
    let (status, offset) = match tz.from_local_datetime(&naive) {
        LocalResult::Single(v) => (
            "single".to_string(),
            Some(v.offset().fix().local_minus_utc()),
        ),
        LocalResult::Ambiguous(a, b) => (
            format!(
                "ambiguous ({} / {} seconds)",
                a.offset().fix().local_minus_utc(),
                b.offset().fix().local_minus_utc()
            ),
            None,
        ),
        LocalResult::None => ("nonexistent".to_string(), None),
    };
    let iso = d.iso_week();
    let leap = is_leap_year(d.year());
    let result = DateInspection {
        gregorian_date: d.to_string(),
        timezone: tz.to_string(),
        year: d.year(),
        month: d.month(),
        day: d.day(),
        weekday: d.format("%A").to_string(),
        quarter: (d.month() - 1) / 3 + 1,
        ordinal_day: d.ordinal(),
        days_in_month: days_in_month(d.year(), d.month())?,
        days_in_year: if leap { 366 } else { 365 },
        days_remaining: (if leap { 366 } else { 365 }) - d.ordinal(),
        is_leap_year: leap,
        iso_week_year: iso.year(),
        iso_week: iso.week(),
        iso_weekday: d.weekday().number_from_monday(),
        local_midnight_status: status,
        utc_offset_seconds: offset,
        utc_offset: offset.map(offset_string),
    };
    match format {
        OutputFormat::Json => emit(&result, format),
        OutputFormat::Text => {
            println!("ChronoGrid Date Inspection");
            println!("Gregorian date: {}", result.gregorian_date);
            println!("Weekday: {}", result.weekday);
            println!(
                "Year/month/day: {}-{:02}-{:02}",
                result.year, result.month, result.day
            );
            println!(
                "Quarter: Q{}; ordinal: {:03}; month length: {}",
                result.quarter, result.ordinal_day, result.days_in_month
            );
            println!(
                "Leap year: {}; days remaining: {}",
                result.is_leap_year, result.days_remaining
            );
            println!(
                "ISO week date: {}-W{:02}-{}",
                result.iso_week_year, result.iso_week, result.iso_weekday
            );
            println!(
                "Timezone: {}; local midnight: {}; UTC offset: {}",
                result.timezone,
                result.local_midnight_status,
                result.utc_offset.as_deref().unwrap_or("context-dependent")
            );
            Ok(())
        }
    }
}
fn week_inspect(input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let i = d.iso_week();
    let r = WeekInspection {
        gregorian_date: d.to_string(),
        iso_week_year: i.year(),
        iso_week: i.week(),
        iso_weekday: d.weekday().number_from_monday(),
    };
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!(
                "Gregorian date: {}\nISO week: {}-W{:02}-{}",
                r.gregorian_date, r.iso_week_year, r.iso_week, r.iso_weekday
            );
            Ok(())
        }
    }
}
fn calendar_data(y: i32, m: u32, start: WeekStart) -> Result<CalendarMonth> {
    let first = NaiveDate::from_ymd_opt(y, m, 1).context("invalid year/month")?;
    let count = days_in_month(y, m)?;
    let leading = (first.weekday().number_from_monday() + 7 - start.number()) % 7;
    let rows = (leading + count).div_ceil(7);
    let begin = first - Duration::days(i64::from(leading));
    let weekdays = if matches!(start, WeekStart::Monday) {
        vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    } else {
        vec!["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    }
    .into_iter()
    .map(str::to_string)
    .collect();
    let mut weeks = Vec::new();
    for w in 0..rows {
        let mut row = Vec::new();
        for d in 0..7 {
            let date = begin + Duration::days(i64::from(w * 7 + d));
            row.push(CalendarDay {
                date: date.to_string(),
                day: date.day(),
                in_month: date.month() == m,
            });
        }
        weeks.push(row);
    }
    Ok(CalendarMonth {
        year: y,
        month: m,
        week_start: format!("{:?}", start).to_lowercase(),
        weekdays,
        weeks,
    })
}
fn calendar_month(y: i32, m: u32, start: WeekStart, format: OutputFormat) -> Result<()> {
    let c = calendar_data(y, m, start)?;
    match format {
        OutputFormat::Json => emit(&c, format),
        OutputFormat::Text => {
            println!("{} {} (week starts {})", c.month, c.year, c.week_start);
            for d in &c.weekdays {
                print!("{d:>4}");
            }
            println!();
            for row in &c.weeks {
                for day in row {
                    if day.in_month {
                        print!("{:>4}", day.day)
                    } else {
                        print!("    ")
                    }
                }
                println!()
            }
            Ok(())
        }
    }
}
fn jd(date: NaiveDateTime) -> f64 {
    let jdn = i64::from(date.date().num_days_from_ce()) + 1_721_425;
    let seconds = f64::from(date.time().num_seconds_from_midnight());
    jdn as f64 - 0.5 + seconds / 86_400.0
}
fn convert_gregorian(input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let midnight = d.and_hms_opt(0, 0, 0).unwrap();
    let noon = d.and_hms_opt(12, 0, 0).unwrap();
    let iso = d.iso_week();
    let r=Conversion{gregorian_date:d.to_string(),ordinal_date:format!("{}-{:03}",d.year(),d.ordinal()),iso_week_date:format!("{}-W{:02}-{}",iso.year(),iso.week(),d.weekday().number_from_monday()),astronomical_julian_date_at_utc_midnight:jd(midnight),julian_day_number_at_utc_noon:jd(noon).floor() as i64,modified_julian_date_at_utc_midnight:jd(midnight)-2_400_000.5,terminology_note:"Ordinal date, astronomical Julian Date (JD), integer Julian Day Number (JDN), and Modified Julian Date (MJD) are distinct systems.".into()};
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("Gregorian civil date: {}", r.gregorian_date);
            println!("Ordinal date: {}", r.ordinal_date);
            println!("ISO week date: {}", r.iso_week_date);
            println!(
                "Astronomical JD at UTC midnight: {:.1}",
                r.astronomical_julian_date_at_utc_midnight
            );
            println!(
                "Julian Day Number at UTC noon: {}",
                r.julian_day_number_at_utc_noon
            );
            println!(
                "Modified Julian Date at UTC midnight: {:.1}",
                r.modified_julian_date_at_utc_midnight
            );
            Ok(())
        }
    }
}
fn utc_offset(tz: Tz, utc: NaiveDateTime) -> i32 {
    tz.from_utc_datetime(&utc).offset().fix().local_minus_utc()
}
fn transitions(tz: Tz, year: i32) -> Result<Vec<Transition>> {
    if !(1..=9998).contains(&year) {
        bail!("year must be between 1 and 9998");
    }
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut out = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let next = (cursor + Duration::hours(6)).min(end);
        let old = utc_offset(tz, cursor);
        let new = utc_offset(tz, next);
        if old != new {
            let mut lo = cursor;
            let mut hi = next;
            while (hi - lo).num_seconds() > 1 {
                let mid = lo + Duration::seconds((hi - lo).num_seconds() / 2);
                if utc_offset(tz, mid) == old {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            let when = DateTime::<Utc>::from_naive_utc_and_offset(hi, Utc);
            let local = tz.from_utc_datetime(&hi);
            out.push(Transition {
                utc: when.to_rfc3339(),
                local: local.to_rfc3339(),
                old_offset_seconds: old,
                new_offset_seconds: new,
                old_offset: offset_string(old),
                new_offset: offset_string(new),
                change: if new > old { "forward" } else { "backward" }.into(),
            });
        }
        cursor = next;
    }
    Ok(out)
}
fn timezone_inspect(zone: &str, year: i32, format: OutputFormat) -> Result<()> {
    let tz = parse_tz(zone)?;
    let r = TransitionReport {
        timezone: tz.to_string(),
        year,
        transitions: transitions(tz, year)?,
    };
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("Timezone transitions: {} ({})", r.timezone, r.year);
            if r.transitions.is_empty() {
                println!("No offset changes in this year.");
            }
            for t in &r.transitions {
                println!(
                    "{} local {}: {} -> {} ({})",
                    t.utc, t.local, t.old_offset, t.new_offset, t.change
                )
            }
            Ok(())
        }
    }
}
fn challenge_vectors() -> Vec<ChallengeVector> {
    let mut v = Vec::new();
    let mut push = |id: &str, class: &str, passed: bool| {
        v.push(ChallengeVector {
            id: id.into(),
            class: class.into(),
            passed,
        })
    };
    let jan = parse_date("2027-01-01").unwrap();
    let iso = jan.iso_week();
    push(
        "ISO-2027-01-01",
        "ISO-WEEK-YEAR",
        iso.year() == 2026 && iso.week() == 53 && jan.weekday().number_from_monday() == 5,
    );
    push(
        "LEAP-2000",
        "GREGORIAN-LEAP",
        is_leap_year(2000) && !is_leap_year(1900),
    );
    push("ORDINAL-2027-001", "ORDINAL-DATE", jan.ordinal() == 1);
    push(
        "MONTH-2028-02",
        "END-OF-MONTH",
        matches!(days_in_month(2028, 2), Ok(29)),
    );
    push(
        "JD-1970-01-01",
        "JULIAN-DAY",
        (jd(NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap())
            - 2_440_587.5)
            .abs()
            < 1e-9,
    );
    push(
        "JD-2000-01-01T12",
        "JULIAN-DAY",
        (jd(NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap())
            - 2_451_545.0)
            .abs()
            < 1e-9,
    );
    let ny = parse_tz("America/New_York").unwrap();
    let tr = transitions(ny, 2027).unwrap();
    push(
        "NY-DST-2027",
        "DST-BOUNDARY",
        tr.len() == 2 && tr[0].change == "forward" && tr[1].change == "backward",
    );
    let ind = parse_tz("Asia/Kolkata").unwrap();
    let nonwhole = utc_offset(
        ind,
        NaiveDate::from_ymd_opt(2027, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    push(
        "OFFSET-INDIA",
        "TIMEZONE-OFFSET",
        nonwhole == 19_800 && transitions(ind, 2027).unwrap().is_empty(),
    );
    let mut matrix_ok = true;
    for y in 2026..=2028 {
        for m in 1..=12 {
            let c = calendar_data(y, m, WeekStart::Monday).unwrap();
            let count = c.weeks.iter().flatten().filter(|d| d.in_month).count();
            matrix_ok &=
                count == days_in_month(y, m).unwrap() as usize && (4..=6).contains(&c.weeks.len());
        }
    }
    push("GRID-INVARIANTS", "CALENDAR-GRID", matrix_ok);
    v
}
fn challenge(format: OutputFormat, report: Option<PathBuf>) -> Result<()> {
    let vectors = challenge_vectors();
    let passed = vectors.iter().filter(|v| v.passed).count();
    let r = ChallengeReport {
        version: env!("CARGO_PKG_VERSION").into(),
        vectors_executed: vectors.len(),
        passed,
        failed: vectors.len() - passed,
        vectors,
        reproduction_command: "chronogrid challenge".into(),
    };
    if let Some(ref path) = report {
        let mut html=String::from("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>ChronoGrid conformance report</title><h1>ChronoGrid Temporal Conformance</h1>");
        html.push_str(&format!("<p>Version {} · vectors {} · passed {} · failed {}</p><p>Reproduce with <code>chronogrid challenge</code></p><ul>",r.version,r.vectors_executed,r.passed,r.failed));
        for v in &r.vectors {
            html.push_str(&format!(
                "<li>{}: {} — {}</li>",
                v.id,
                v.class,
                if v.passed { "PASS" } else { "FAIL" }
            ));
        }
        html.push_str("</ul></html>");
        std::fs::write(path, html).context("could not write HTML report")?;
    }
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("ChronoGrid Temporal Conformance");
            for v in &r.vectors {
                println!("{:<24} {}", v.class, if v.passed { "PASS" } else { "FAIL" });
            }
            println!(
                "Vectors executed: {}\nFailures: {}",
                r.vectors_executed, r.failed
            );
            if let Some(p) = &report {
                println!("HTML report: {}", p.display());
            }
            if r.failed > 0 {
                bail!("{} conformance vector(s) failed", r.failed)
            }
            Ok(())
        }
    }
}
fn rng_next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}
fn chaos(from: i32, to: i32, seed: &str, count: usize, format: OutputFormat) -> Result<()> {
    if from < 1 || to > 9998 || from > to {
        bail!("year range must satisfy 1 <= --from <= --to <= 9998");
    }
    if count == 0 || count > 10_000 {
        bail!("--count must be between 1 and 10000");
    }
    let mut state = seed.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    if state == 0 {
        state = 1;
    }
    let zones = [
        "America/New_York",
        "Europe/London",
        "Australia/Sydney",
        "Asia/Kathmandu",
    ];
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let pick = rng_next(&mut state);
        let year = from + (pick % (u64::from((to - from) as u32) + 1)) as i32;
        let kind = i % 6;
        let (class, date, zone, note) = match kind {
            0 => {
                let leap = (from..=to).find(|y| is_leap_year(*y));
                if let Some(y) = leap {
                    (
                        "GREGORIAN-LEAP",
                        format!("{y}-02-29"),
                        "UTC",
                        "leap day".to_string(),
                    )
                } else {
                    (
                        "YEAR-END",
                        format!("{year}-12-31"),
                        "UTC",
                        "year boundary".to_string(),
                    )
                }
            }
            1 => {
                let m = (rng_next(&mut state) % 12 + 1) as u32;
                (
                    "END-OF-MONTH",
                    format!("{year}-{m:02}-{:02}", days_in_month(year, m)?),
                    "UTC",
                    "month end".to_string(),
                )
            }
            2 => (
                "YEAR-END",
                format!("{year}-12-31"),
                "UTC",
                "Gregorian year end".to_string(),
            ),
            3 => {
                let d = if pick & 1 == 0 {
                    NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
                };
                (
                    "ISO-WEEK-YEAR",
                    d.to_string(),
                    "UTC",
                    "ISO week-year boundary".to_string(),
                )
            }
            4 => {
                let zone = zones[(rng_next(&mut state) % zones.len() as u64) as usize];
                let tz = parse_tz(zone)?;
                let ts = transitions(tz, year)?;
                if let Some(t) = ts.first() {
                    (
                        "DST-BOUNDARY",
                        t.local[..10].to_string(),
                        zone,
                        "timezone offset transition".to_string(),
                    )
                } else {
                    (
                        "TIMEZONE-OFFSET",
                        format!("{year}-01-01"),
                        zone,
                        "zone has no offset change in selected year".to_string(),
                    )
                }
            }
            _ => {
                let month = (rng_next(&mut state) % 12 + 1) as u32;
                (
                    "TIMEZONE-OFFSET",
                    format!("{year}-{month:02}-01"),
                    "Asia/Kathmandu",
                    "non-whole-hour timezone offset".to_string(),
                )
            }
        };
        out.push(ChaosVector {
            id: format!("CG-{i:05}"),
            class: class.into(),
            date,
            timezone: zone.into(),
            note,
        });
    }
    match format {
        OutputFormat::Json => emit(&out, format),
        OutputFormat::Text => {
            println!("Deterministic chaos vectors (seed: {seed})");
            for v in &out {
                println!(
                    "{} {} {} {} — {}",
                    v.id, v.class, v.date, v.timezone, v.note
                );
            }
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_iso_boundary() {
        let d = parse_date("2027-01-01").unwrap();
        assert_eq!(
            (
                d.iso_week().year(),
                d.iso_week().week(),
                d.weekday().number_from_monday()
            ),
            (2026, 53, 5)
        );
    }
    #[test]
    fn leap_rules() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2027));
    }
    #[test]
    fn month_lengths() {
        assert_eq!(days_in_month(2028, 2).unwrap(), 29);
        assert_eq!(days_in_month(2027, 2).unwrap(), 28);
        assert_eq!(days_in_month(2027, 12).unwrap(), 31);
    }
    #[test]
    fn julian_anchors() {
        let a = jd(parse_date("1970-01-01")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap());
        let b = jd(parse_date("2000-01-01")
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap());
        assert_eq!(a, 2_440_587.5);
        assert_eq!(b, 2_451_545.0);
    }
    #[test]
    fn timezone_transition_edges() {
        let t = transitions(parse_tz("America/New_York").unwrap(), 2027).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].change, "forward");
        assert_eq!(t[1].change, "backward");
    }
    #[test]
    fn zones_without_dst_and_half_hour_offsets() {
        let z = parse_tz("Asia/Kolkata").unwrap();
        assert!(transitions(z, 2027).unwrap().is_empty());
        assert_eq!(
            utc_offset(
                z,
                parse_date("2027-01-01")
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            ),
            19_800
        );
    }
    #[test]
    fn calendar_matrices_cover_each_day() {
        for start in [WeekStart::Monday, WeekStart::Sunday] {
            for y in 2026..=2028 {
                for m in 1..=12 {
                    let c = calendar_data(y, m, start).unwrap();
                    assert!((4..=6).contains(&c.weeks.len()));
                    assert_eq!(
                        c.weeks.iter().flatten().filter(|d| d.in_month).count(),
                        days_in_month(y, m).unwrap() as usize
                    );
                }
            }
        }
    }
    #[test]
    fn challenge_has_real_vectors_and_passes() {
        let v = challenge_vectors();
        assert!(v.len() >= 8);
        assert!(v.iter().all(|x| x.passed));
    }
    #[test]
    fn deterministic_rng_repeats() {
        let mut a = 42;
        let mut b = 42;
        for _ in 0..100 {
            assert_eq!(rng_next(&mut a), rng_next(&mut b));
        }
    }
}use anyhow::{bail, Context, Result};
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone,
    Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[command(
    name = "chronogrid",
    version,
    about = "Can your software survive time?",
    long_about = "A deterministic temporal and calendar conformance laboratory."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Date {
        #[command(subcommand)]
        command: DateCommand,
    },
    Week {
        #[command(subcommand)]
        command: WeekCommand,
    },
    Calendar {
        #[command(subcommand)]
        command: CalendarCommand,
    },
    Convert {
        system: String,
        date: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
    Timezone {
        #[command(subcommand)]
        command: TimezoneCommand,
    },
    Challenge {
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    Chaos {
        #[arg(long = "from", default_value_t = 1900)]
        from_year: i32,
        #[arg(long = "to", default_value_t = 2100)]
        to_year: i32,
        #[arg(long, default_value = "chronogrid-demo")]
        seed: String,
        #[arg(long, default_value_t = 25)]
        count: usize,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum DateCommand {
    Inspect {
        date: String,
        #[arg(long, default_value = "UTC")]
        timezone: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum WeekCommand {
    Inspect {
        date: String,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum CalendarCommand {
    Month {
        year: i32,
        month: u32,
        #[arg(long, value_enum, default_value_t=WeekStart::Monday)]
        week_start: WeekStart,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Subcommand)]
enum TimezoneCommand {
    Inspect {
        zone: String,
        #[arg(long)]
        year: i32,
        #[arg(long, value_enum, default_value_t=OutputFormat::Text)]
        format: OutputFormat,
    },
}
#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
#[derive(Copy, Clone, Debug, ValueEnum)]
enum WeekStart {
    Monday,
    Sunday,
}
impl WeekStart {
    fn number(self) -> u32 {
        match self {
            Self::Monday => 1,
            Self::Sunday => 7,
        }
    }
}
#[derive(Debug, Serialize)]
struct DateInspection {
    gregorian_date: String,
    timezone: String,
    year: i32,
    month: u32,
    day: u32,
    weekday: String,
    quarter: u32,
    ordinal_day: u32,
    days_in_month: u32,
    days_in_year: u32,
    days_remaining: u32,
    is_leap_year: bool,
    iso_week_year: i32,
    iso_week: u32,
    iso_weekday: u32,
    local_midnight_status: String,
    utc_offset_seconds: Option<i32>,
    utc_offset: Option<String>,
}
#[derive(Debug, Serialize)]
struct WeekInspection {
    gregorian_date: String,
    iso_week_year: i32,
    iso_week: u32,
    iso_weekday: u32,
}
#[derive(Debug, Serialize)]
struct CalendarDay {
    date: String,
    day: u32,
    in_month: bool,
}
#[derive(Debug, Serialize)]
struct CalendarMonth {
    year: i32,
    month: u32,
    week_start: String,
    weekdays: Vec<String>,
    weeks: Vec<Vec<CalendarDay>>,
}
#[derive(Debug, Serialize)]
struct Conversion {
    gregorian_date: String,
    ordinal_date: String,
    iso_week_date: String,
    astronomical_julian_date_at_utc_midnight: f64,
    julian_day_number_at_utc_noon: i64,
    modified_julian_date_at_utc_midnight: f64,
    terminology_note: String,
}
#[derive(Debug, Serialize)]
struct Transition {
    utc: String,
    local: String,
    old_offset_seconds: i32,
    new_offset_seconds: i32,
    old_offset: String,
    new_offset: String,
    change: String,
}
#[derive(Debug, Serialize)]
struct TransitionReport {
    timezone: String,
    year: i32,
    transitions: Vec<Transition>,
}
#[derive(Debug, Serialize)]
struct ChallengeVector {
    id: String,
    class: String,
    passed: bool,
}
#[derive(Debug, Serialize)]
struct ChallengeReport {
    version: String,
    vectors_executed: usize,
    passed: usize,
    failed: usize,
    vectors: Vec<ChallengeVector>,
    reproduction_command: String,
}
#[derive(Debug, Serialize)]
struct ChaosVector {
    id: String,
    class: String,
    date: String,
    timezone: String,
    note: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Date {
            command:
                DateCommand::Inspect {
                    date,
                    timezone,
                    format,
                },
        } => date_inspect(&date, &timezone, format),
        Commands::Week {
            command: WeekCommand::Inspect { date, format },
        } => week_inspect(&date, format),
        Commands::Calendar {
            command:
                CalendarCommand::Month {
                    year,
                    month,
                    week_start,
                    format,
                },
        } => calendar_month(year, month, week_start, format),
        Commands::Convert {
            system,
            date,
            format,
        } => {
            if system.to_lowercase() != "gregorian" {
                bail!("only the Gregorian civil calendar is supported by convert");
            }
            convert_gregorian(&date, format)
        }
        Commands::Timezone {
            command: TimezoneCommand::Inspect { zone, year, format },
        } => timezone_inspect(&zone, year, format),
        Commands::Challenge { format, report } => challenge(format, report),
        Commands::Chaos {
            from_year,
            to_year,
            seed,
            count,
            format,
        } => chaos(from_year, to_year, &seed, count, format),
    }
}
fn parse_date(input: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .with_context(|| format!("invalid Gregorian date '{input}'; expected YYYY-MM-DD"))
}
fn parse_tz(input: &str) -> Result<Tz> {
    Tz::from_str(input).map_err(|_| anyhow::anyhow!("unknown IANA timezone '{input}'"))
}
fn is_leap_year(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
fn days_in_month(y: i32, m: u32) -> Result<u32> {
    if !(1..=12).contains(&m) {
        bail!("month must be between 1 and 12");
    }
    let next = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
    }
    .context("year is outside the supported date range")?;
    Ok(next
        .pred_opt()
        .context("date outside supported range")?
        .day())
}
fn offset_string(s: i32) -> String {
    let sign = if s < 0 { '-' } else { '+' };
    let a = s.abs();
    format!("{sign}{:02}:{:02}", a / 3600, (a % 3600) / 60)
}
fn emit<T: Serialize>(value: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
        OutputFormat::Text => println!("{}", serde_json::to_string_pretty(value)?),
    }
    Ok(())
}
fn date_inspect(input: &str, zone_input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let tz = parse_tz(zone_input)?;
    let naive = d.and_hms_opt(0, 0, 0).context("invalid local midnight")?;
    let (status, offset) = match tz.from_local_datetime(&naive) {
        LocalResult::Single(v) => (
            "single".to_string(),
            Some(v.offset().fix().local_minus_utc()),
        ),
        LocalResult::Ambiguous(a, b) => (
            format!(
                "ambiguous ({} / {} seconds)",
                a.offset().fix().local_minus_utc(),
                b.offset().fix().local_minus_utc()
            ),
            None,
        ),
        LocalResult::None => ("nonexistent".to_string(), None),
    };
    let iso = d.iso_week();
    let leap = is_leap_year(d.year());
    let result = DateInspection {
        gregorian_date: d.to_string(),
        timezone: tz.to_string(),
        year: d.year(),
        month: d.month(),
        day: d.day(),
        weekday: d.format("%A").to_string(),
        quarter: (d.month() - 1) / 3 + 1,
        ordinal_day: d.ordinal(),
        days_in_month: days_in_month(d.year(), d.month())?,
        days_in_year: if leap { 366 } else { 365 },
        days_remaining: (if leap { 366 } else { 365 }) - d.ordinal(),
        is_leap_year: leap,
        iso_week_year: iso.year(),
        iso_week: iso.week(),
        iso_weekday: d.weekday().number_from_monday(),
        local_midnight_status: status,
        utc_offset_seconds: offset,
        utc_offset: offset.map(offset_string),
    };
    match format {
        OutputFormat::Json => emit(&result, format),
        OutputFormat::Text => {
            println!("ChronoGrid Date Inspection");
            println!("Gregorian date: {}", result.gregorian_date);
            println!("Weekday: {}", result.weekday);
            println!(
                "Year/month/day: {}-{:02}-{:02}",
                result.year, result.month, result.day
            );
            println!(
                "Quarter: Q{}; ordinal: {:03}; month length: {}",
                result.quarter, result.ordinal_day, result.days_in_month
            );
            println!(
                "Leap year: {}; days remaining: {}",
                result.is_leap_year, result.days_remaining
            );
            println!(
                "ISO week date: {}-W{:02}-{}",
                result.iso_week_year, result.iso_week, result.iso_weekday
            );
            println!(
                "Timezone: {}; local midnight: {}; UTC offset: {}",
                result.timezone,
                result.local_midnight_status,
                result.utc_offset.as_deref().unwrap_or("context-dependent")
            );
            Ok(())
        }
    }
}
fn week_inspect(input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let i = d.iso_week();
    let r = WeekInspection {
        gregorian_date: d.to_string(),
        iso_week_year: i.year(),
        iso_week: i.week(),
        iso_weekday: d.weekday().number_from_monday(),
    };
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!(
                "Gregorian date: {}\nISO week: {}-W{:02}-{}",
                r.gregorian_date, r.iso_week_year, r.iso_week, r.iso_weekday
            );
            Ok(())
        }
    }
}
fn calendar_data(y: i32, m: u32, start: WeekStart) -> Result<CalendarMonth> {
    let first = NaiveDate::from_ymd_opt(y, m, 1).context("invalid year/month")?;
    let count = days_in_month(y, m)?;
    let leading = (first.weekday().number_from_monday() + 7 - start.number()) % 7;
    let rows = (leading + count + 6) / 7;
    let begin = first - Duration::days(i64::from(leading));
    let weekdays = if matches!(start, WeekStart::Monday) {
        vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    } else {
        vec!["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    }
    .into_iter()
    .map(str::to_string)
    .collect();
    let mut weeks = Vec::new();
    for w in 0..rows {
        let mut row = Vec::new();
        for d in 0..7 {
            let date = begin + Duration::days(i64::from(w * 7 + d));
            row.push(CalendarDay {
                date: date.to_string(),
                day: date.day(),
                in_month: date.month() == m,
            });
        }
        weeks.push(row);
    }
    Ok(CalendarMonth {
        year: y,
        month: m,
        week_start: format!("{:?}", start).to_lowercase(),
        weekdays,
        weeks,
    })
}
fn calendar_month(y: i32, m: u32, start: WeekStart, format: OutputFormat) -> Result<()> {
    let c = calendar_data(y, m, start)?;
    match format {
        OutputFormat::Json => emit(&c, format),
        OutputFormat::Text => {
            println!("{} {} (week starts {})", c.month, c.year, c.week_start);
            for d in &c.weekdays {
                print!("{d:>4}");
            }
            println!();
            for row in &c.weeks {
                for day in row {
                    if day.in_month {
                        print!("{:>4}", day.day)
                    } else {
                        print!("    ")
                    }
                }
                println!()
            }
            Ok(())
        }
    }
}
fn jd(date: NaiveDateTime) -> f64 {
    let jdn = i64::from(date.date().num_days_from_ce()) + 1_721_425;
    let seconds = f64::from(date.time().num_seconds_from_midnight());
    jdn as f64 - 0.5 + seconds / 86_400.0
}
fn convert_gregorian(input: &str, format: OutputFormat) -> Result<()> {
    let d = parse_date(input)?;
    let midnight = d.and_hms_opt(0, 0, 0).unwrap();
    let noon = d.and_hms_opt(12, 0, 0).unwrap();
    let iso = d.iso_week();
    let r=Conversion{gregorian_date:d.to_string(),ordinal_date:format!("{}-{:03}",d.year(),d.ordinal()),iso_week_date:format!("{}-W{:02}-{}",iso.year(),iso.week(),d.weekday().number_from_monday()),astronomical_julian_date_at_utc_midnight:jd(midnight),julian_day_number_at_utc_noon:jd(noon).floor() as i64,modified_julian_date_at_utc_midnight:jd(midnight)-2_400_000.5,terminology_note:"Ordinal date, astronomical Julian Date (JD), integer Julian Day Number (JDN), and Modified Julian Date (MJD) are distinct systems.".into()};
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("Gregorian civil date: {}", r.gregorian_date);
            println!("Ordinal date: {}", r.ordinal_date);
            println!("ISO week date: {}", r.iso_week_date);
            println!(
                "Astronomical JD at UTC midnight: {:.1}",
                r.astronomical_julian_date_at_utc_midnight
            );
            println!(
                "Julian Day Number at UTC noon: {}",
                r.julian_day_number_at_utc_noon
            );
            println!(
                "Modified Julian Date at UTC midnight: {:.1}",
                r.modified_julian_date_at_utc_midnight
            );
            Ok(())
        }
    }
}
fn utc_offset(tz: Tz, utc: NaiveDateTime) -> i32 {
    tz.from_utc_datetime(&utc).offset().fix().local_minus_utc()
}
fn transitions(tz: Tz, year: i32) -> Result<Vec<Transition>> {
    if !(1..=9998).contains(&year) {
        bail!("year must be between 1 and 9998");
    }
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut out = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let next = (cursor + Duration::hours(6)).min(end);
        let old = utc_offset(tz, cursor);
        let new = utc_offset(tz, next);
        if old != new {
            let mut lo = cursor;
            let mut hi = next;
            while (hi - lo).num_seconds() > 1 {
                let mid = lo + Duration::seconds((hi - lo).num_seconds() / 2);
                if utc_offset(tz, mid) == old {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            let when = DateTime::<Utc>::from_naive_utc_and_offset(hi, Utc);
            let local = tz.from_utc_datetime(&hi);
            out.push(Transition {
                utc: when.to_rfc3339(),
                local: local.to_rfc3339(),
                old_offset_seconds: old,
                new_offset_seconds: new,
                old_offset: offset_string(old),
                new_offset: offset_string(new),
                change: if new > old { "forward" } else { "backward" }.into(),
            });
        }
        cursor = next;
    }
    Ok(out)
}
fn timezone_inspect(zone: &str, year: i32, format: OutputFormat) -> Result<()> {
    let tz = parse_tz(zone)?;
    let r = TransitionReport {
        timezone: tz.to_string(),
        year,
        transitions: transitions(tz, year)?,
    };
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("Timezone transitions: {} ({})", r.timezone, r.year);
            if r.transitions.is_empty() {
                println!("No offset changes in this year.");
            }
            for t in &r.transitions {
                println!(
                    "{} local {}: {} -> {} ({})",
                    t.utc, t.local, t.old_offset, t.new_offset, t.change
                )
            }
            Ok(())
        }
    }
}
fn challenge_vectors() -> Vec<ChallengeVector> {
    let mut v = Vec::new();
    let mut push = |id: &str, class: &str, passed: bool| {
        v.push(ChallengeVector {
            id: id.into(),
            class: class.into(),
            passed,
        })
    };
    let jan = parse_date("2027-01-01").unwrap();
    let iso = jan.iso_week();
    push(
        "ISO-2027-01-01",
        "ISO-WEEK-YEAR",
        iso.year() == 2026 && iso.week() == 53 && jan.weekday().number_from_monday() == 5,
    );
    push(
        "LEAP-2000",
        "GREGORIAN-LEAP",
        is_leap_year(2000) && !is_leap_year(1900),
    );
    push("ORDINAL-2027-001", "ORDINAL-DATE", jan.ordinal() == 1);
    push(
        "MONTH-2028-02",
        "END-OF-MONTH",
        matches!(days_in_month(2028, 2), Ok(29)),
    );
    push(
        "JD-1970-01-01",
        "JULIAN-DAY",
        (jd(NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap())
            - 2_440_587.5)
            .abs()
            < 1e-9,
    );
    push(
        "JD-2000-01-01T12",
        "JULIAN-DAY",
        (jd(NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap())
            - 2_451_545.0)
            .abs()
            < 1e-9,
    );
    let ny = parse_tz("America/New_York").unwrap();
    let tr = transitions(ny, 2027).unwrap();
    push(
        "NY-DST-2027",
        "DST-BOUNDARY",
        tr.len() == 2 && tr[0].change == "forward" && tr[1].change == "backward",
    );
    let ind = parse_tz("Asia/Kolkata").unwrap();
    let nonwhole = utc_offset(
        ind,
        NaiveDate::from_ymd_opt(2027, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    push(
        "OFFSET-INDIA",
        "TIMEZONE-OFFSET",
        nonwhole == 19_800 && transitions(ind, 2027).unwrap().is_empty(),
    );
    let mut matrix_ok = true;
    for y in 2026..=2028 {
        for m in 1..=12 {
            let c = calendar_data(y, m, WeekStart::Monday).unwrap();
            let count = c.weeks.iter().flatten().filter(|d| d.in_month).count();
            matrix_ok &=
                count == days_in_month(y, m).unwrap() as usize && (4..=6).contains(&c.weeks.len());
        }
    }
    push("GRID-INVARIANTS", "CALENDAR-GRID", matrix_ok);
    v
}
fn challenge(format: OutputFormat, report: Option<PathBuf>) -> Result<()> {
    let vectors = challenge_vectors();
    let passed = vectors.iter().filter(|v| v.passed).count();
    let r = ChallengeReport {
        version: env!("CARGO_PKG_VERSION").into(),
        vectors_executed: vectors.len(),
        passed,
        failed: vectors.len() - passed,
        vectors,
        reproduction_command: "chronogrid challenge".into(),
    };
    if let Some(ref path) = report {
        let mut html=String::from("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>ChronoGrid conformance report</title><h1>ChronoGrid Temporal Conformance</h1>");
        html.push_str(&format!("<p>Version {} · vectors {} · passed {} · failed {}</p><p>Reproduce with <code>chronogrid challenge</code></p><ul>",r.version,r.vectors_executed,r.passed,r.failed));
        for v in &r.vectors {
            html.push_str(&format!(
                "<li>{}: {} — {}</li>",
                v.id,
                v.class,
                if v.passed { "PASS" } else { "FAIL" }
            ));
        }
        html.push_str("</ul></html>");
        std::fs::write(path, html).context("could not write HTML report")?;
    }
    match format {
        OutputFormat::Json => emit(&r, format),
        OutputFormat::Text => {
            println!("ChronoGrid Temporal Conformance");
            for v in &r.vectors {
                println!("{:<24} {}", v.class, if v.passed { "PASS" } else { "FAIL" });
            }
            println!(
                "Vectors executed: {}\nFailures: {}",
                r.vectors_executed, r.failed
            );
            if let Some(p) = &report {
                println!("HTML report: {}", p.display());
            }
            if r.failed > 0 {
                bail!("{} conformance vector(s) failed", r.failed)
            }
            Ok(())
        }
    }
}
fn rng_next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}
fn chaos(from: i32, to: i32, seed: &str, count: usize, format: OutputFormat) -> Result<()> {
    if from < 1 || to > 9998 || from > to {
        bail!("year range must satisfy 1 <= --from <= --to <= 9998");
    }
    if count == 0 || count > 10_000 {
        bail!("--count must be between 1 and 10000");
    }
    let mut state = seed.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    if state == 0 {
        state = 1;
    }
    let zones = [
        "America/New_York",
        "Europe/London",
        "Australia/Sydney",
        "Asia/Kathmandu",
    ];
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let pick = rng_next(&mut state);
        let year = from + (pick % (u64::from((to - from) as u32) + 1)) as i32;
        let kind = i % 6;
        let (class, date, zone, note) = match kind {
            0 => {
                let leap = (from..=to).find(|y| is_leap_year(*y));
                if let Some(y) = leap {
                    (
                        "GREGORIAN-LEAP",
                        format!("{y}-02-29"),
                        "UTC",
                        "leap day".to_string(),
                    )
                } else {
                    (
                        "YEAR-END",
                        format!("{year}-12-31"),
                        "UTC",
                        "year boundary".to_string(),
                    )
                }
            }
            1 => {
                let m = (rng_next(&mut state) % 12 + 1) as u32;
                (
                    "END-OF-MONTH",
                    format!("{year}-{m:02}-{:02}", days_in_month(year, m)?),
                    "UTC",
                    "month end".to_string(),
                )
            }
            2 => (
                "YEAR-END",
                format!("{year}-12-31"),
                "UTC",
                "Gregorian year end".to_string(),
            ),
            3 => {
                let d = if pick & 1 == 0 {
                    NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
                };
                (
                    "ISO-WEEK-YEAR",
                    d.to_string(),
                    "UTC",
                    "ISO week-year boundary".to_string(),
                )
            }
            4 => {
                let zone = zones[(rng_next(&mut state) % zones.len() as u64) as usize];
                let tz = parse_tz(zone)?;
                let ts = transitions(tz, year)?;
                if let Some(t) = ts.first() {
                    (
                        "DST-BOUNDARY",
                        t.local[..10].to_string(),
                        zone,
                        "timezone offset transition".to_string(),
                    )
                } else {
                    (
                        "TIMEZONE-OFFSET",
                        format!("{year}-01-01"),
                        zone,
                        "zone has no offset change in selected year".to_string(),
                    )
                }
            }
            _ => {
                let month = (rng_next(&mut state) % 12 + 1) as u32;
                (
                    "TIMEZONE-OFFSET",
                    format!("{year}-{month:02}-01"),
                    "Asia/Kathmandu",
                    "non-whole-hour timezone offset".to_string(),
                )
            }
        };
        out.push(ChaosVector {
            id: format!("CG-{i:05}"),
            class: class.into(),
            date,
            timezone: zone.into(),
            note,
        });
    }
    match format {
        OutputFormat::Json => emit(&out, format),
        OutputFormat::Text => {
            println!("Deterministic chaos vectors (seed: {seed})");
            for v in &out {
                println!(
                    "{} {} {} {} — {}",
                    v.id, v.class, v.date, v.timezone, v.note
                );
            }
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_iso_boundary() {
        let d = parse_date("2027-01-01").unwrap();
        assert_eq!(
            (
                d.iso_week().year(),
                d.iso_week().week(),
                d.weekday().number_from_monday()
            ),
            (2026, 53, 5)
        );
    }
    #[test]
    fn leap_rules() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2027));
    }
    #[test]
    fn month_lengths() {
        assert_eq!(days_in_month(2028, 2).unwrap(), 29);
        assert_eq!(days_in_month(2027, 2).unwrap(), 28);
        assert_eq!(days_in_month(2027, 12).unwrap(), 31);
    }
    #[test]
    fn julian_anchors() {
        let a = jd(parse_date("1970-01-01")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap());
        let b = jd(parse_date("2000-01-01")
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap());
        assert_eq!(a, 2_440_587.5);
        assert_eq!(b, 2_451_545.0);
    }
    #[test]
    fn timezone_transition_edges() {
        let t = transitions(parse_tz("America/New_York").unwrap(), 2027).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].change, "forward");
        assert_eq!(t[1].change, "backward");
    }
    #[test]
    fn zones_without_dst_and_half_hour_offsets() {
        let z = parse_tz("Asia/Kolkata").unwrap();
        assert!(transitions(z, 2027).unwrap().is_empty());
        assert_eq!(
            utc_offset(
                z,
                parse_date("2027-01-01")
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            ),
            19_800
        );
    }
    #[test]
    fn calendar_matrices_cover_each_day() {
        for start in [WeekStart::Monday, WeekStart::Sunday] {
            for y in 2026..=2028 {
                for m in 1..=12 {
                    let c = calendar_data(y, m, start).unwrap();
                    assert!((4..=6).contains(&c.weeks.len()));
                    assert_eq!(
                        c.weeks.iter().flatten().filter(|d| d.in_month).count(),
                        days_in_month(y, m).unwrap() as usize
                    );
                }
            }
        }
    }
    #[test]
    fn challenge_has_real_vectors_and_passes() {
        let v = challenge_vectors();
        assert!(v.len() >= 8);
        assert!(v.iter().all(|x| x.passed));
    }
    #[test]
    fn deterministic_rng_repeats() {
        let mut a = 42;
        let mut b = 42;
        for _ in 0..100 {
            assert_eq!(rng_next(&mut a), rng_next(&mut b));
        }
    }
}
