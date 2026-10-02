use anyhow::{bail, Context, Result};
use chrono::{Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use rrule::RRuleSet;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    str::FromStr,
};

use crate::legacy::OutputFormat;

#[derive(Clone, Debug)]
struct Property {
    name: String,
    params: HashMap<String, String>,
    value: String,
}
type Event = Vec<Property>;
#[derive(Serialize, Clone)]
struct Issue {
    severity: &'static str,
    code: String,
    component: Option<String>,
    message: String,
}
#[derive(Serialize)]
struct AuditReport {
    schema_version: u8,
    chronogrid_version: &'static str,
    file: String,
    scope: &'static str,
    result: &'static str,
    events: usize,
    issue_counts: BTreeMap<String, usize>,
    issues: Vec<Issue>,
}
#[derive(Serialize)]
struct Occurrence {
    uid: Option<String>,
    summary: Option<String>,
    original_start: String,
    occurrence_start: String,
    occurrence_end: Option<String>,
    timezone: Option<String>,
    recurrence_source: String,
}

fn parse(path: &Path) -> Result<(String, Vec<Event>)> {
    let size = std::fs::metadata(path)
        .with_context(|| format!("cannot inspect {}", path.display()))?
        .len();
    if size > 10 * 1024 * 1024 {
        bail!("ICS input exceeds the 10 MiB diagnostic safety limit");
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut lines: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(prev) = lines.last_mut() {
                prev.push_str(&raw[1..]);
            } else {
                lines.push(raw.to_string());
            }
        } else {
            lines.push(raw.to_string());
        }
    }
    let mut in_calendar = false;
    let mut in_event = false;
    let mut events = Vec::new();
    let mut current = Vec::new();
    let mut has_calendar = false;
    for line in lines.into_iter().filter(|l| !l.trim().is_empty()) {
        let upper = line.to_ascii_uppercase();
        if upper == "BEGIN:VCALENDAR" {
            has_calendar = true;
            in_calendar = true;
            continue;
        }
        if upper == "END:VCALENDAR" {
            in_calendar = false;
            continue;
        }
        if upper == "BEGIN:VEVENT" {
            if !in_calendar {
                bail!("VEVENT outside VCALENDAR");
            }
            in_event = true;
            current.clear();
            continue;
        }
        if upper == "END:VEVENT" {
            if !in_event {
                bail!("END:VEVENT without BEGIN:VEVENT");
            }
            events.push(std::mem::take(&mut current));
            in_event = false;
            continue;
        }
        if !in_event {
            continue;
        }
        let (head, value) = line
            .split_once(':')
            .with_context(|| format!("malformed property line '{line}'"))?;
        let mut parts = head.split(';');
        let name = parts.next().unwrap_or_default().to_ascii_uppercase();
        if name.is_empty() {
            bail!("empty property name");
        }
        let mut params = HashMap::new();
        for part in parts {
            let (k, v) = part
                .split_once('=')
                .with_context(|| format!("malformed parameter in '{line}'"))?;
            params.insert(k.to_ascii_uppercase(), v.trim_matches('"').to_string());
        }
        current.push(Property {
            name,
            params,
            value: value.into(),
        });
    }
    if !has_calendar {
        bail!("missing BEGIN:VCALENDAR");
    }
    if in_event {
        bail!("unterminated VEVENT");
    }
    Ok((text, events))
}
fn values<'a>(e: &'a Event, key: &str) -> Vec<&'a Property> {
    e.iter().filter(|p| p.name == key).collect()
}
fn prop<'a>(e: &'a Event, key: &str) -> Option<&'a Property> {
    e.iter().find(|p| p.name == key)
}
fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}
fn parsed_dt(p: &Property) -> Result<(Option<NaiveDateTime>, bool, Option<Tz>)> {
    let is_date = p
        .params
        .get("VALUE")
        .is_some_and(|v| v.eq_ignore_ascii_case("DATE"))
        || (p.value.len() == 8 && !p.value.contains('T'));
    if is_date {
        let d = NaiveDate::parse_from_str(&p.value, "%Y%m%d")
            .with_context(|| format!("invalid DATE value '{}'; expected YYYYMMDD", p.value))?;
        return Ok((Some(d.and_hms_opt(0, 0, 0).unwrap()), true, None));
    }
    let utc = p.value.ends_with('Z');
    let raw = p.value.trim_end_matches('Z');
    let dt = NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").with_context(|| {
        format!(
            "invalid DATE-TIME value '{}'; expected YYYYMMDDTHHMMSS[Z]",
            p.value
        )
    })?;
    let tz = p
        .params
        .get("TZID")
        .map(|z| Tz::from_str(z).map_err(|_| anyhow::anyhow!("invalid TZID '{z}'")))
        .transpose()?;
    Ok((Some(dt), utc, tz))
}
fn issue(
    out: &mut Vec<Issue>,
    severity: &'static str,
    code: &str,
    component: Option<String>,
    message: impl Into<String>,
) {
    out.push(Issue {
        severity,
        code: code.into(),
        component,
        message: message.into(),
    });
}
fn audit_event(
    e: &Event,
    idx: usize,
    issues: &mut Vec<Issue>,
    uid_counts: &mut HashMap<String, usize>,
) {
    let id = format!("VEVENT[{idx}]");
    let uid = prop(e, "UID").map(|p| p.value.trim().to_string());
    if let Some(uid) = &uid {
        *uid_counts.entry(uid.clone()).or_default() += 1;
    } else {
        issue(
            issues,
            "ERROR",
            "ICS-UID-MISSING",
            Some(id.clone()),
            "VEVENT is missing required UID",
        );
    }
    for required in ["DTSTAMP", "DTSTART"] {
        if prop(e, required).is_none() {
            issue(
                issues,
                "ERROR",
                &format!("ICS-{required}-MISSING"),
                Some(id.clone()),
                format!("VEVENT is missing required {required}"),
            );
        }
    }
    for key in ["DTSTART", "DTEND", "DTSTAMP", "RDATE", "EXDATE"] {
        for p in values(e, key) {
            for value in p.value.split(',') {
                let mut item = p.clone();
                item.value = value.to_string();
                match parsed_dt(&item) {
                    Err(err) => issue(
                        issues,
                        "ERROR",
                        if err.to_string().contains("invalid TZID") {
                            "ICS-TZID-INVALID"
                        } else {
                            "ICS-DATETIME-MALFORMED"
                        },
                        Some(id.clone()),
                        format!("{key}: {err}"),
                    ),
                    Ok((Some(dt), is_date, tz)) => {
                        if key == "DTSTART" && !is_date && !p.value.ends_with('Z') && tz.is_none() {
                            issue(
                                issues,
                                "WARNING",
                                "ICS-FLOATING-DATETIME",
                                Some(id.clone()),
                                "floating local DATE-TIME has no TZID or UTC marker",
                            );
                        }
                        if let Some(tz) = tz {
                            match tz.from_local_datetime(&dt) {
                            LocalResult::Ambiguous(_, _) => issue(issues, "WARNING", "ICS-DST-FOLD", Some(id.clone()), format!("{key} is ambiguous in {tz}")),
                            LocalResult::None => issue(issues, "ERROR", "ICS-DST-GAP", Some(id.clone()), format!("{key} does not exist in {tz} due to a forward clock transition")),
                            LocalResult::Single(_) => {}
                        }
                        }
                    }
                    Ok(_) => {}
                }
            }
        }
    }
    if let (Some(a), Some(b)) = (prop(e, "DTSTART"), prop(e, "DTEND")) {
        if let (Ok((Some(start), sa, _)), Ok((Some(end), sb, _))) = (parsed_dt(a), parsed_dt(b)) {
            if sa != sb {
                issue(
                    issues,
                    "ERROR",
                    "ICS-DATE-TYPE-MISMATCH",
                    Some(id.clone()),
                    "DTSTART and DTEND must both use DATE or both DATE-TIME",
                );
            } else if !sa && end <= start {
                issue(
                    issues,
                    "ERROR",
                    "ICS-END-NOT-AFTER-START",
                    Some(id.clone()),
                    "DTEND must be later than DTSTART",
                );
            } else if sa && end <= start {
                issue(
                    issues,
                    "ERROR",
                    "ICS-END-BEFORE-START",
                    Some(id.clone()),
                    "all-day DTEND is exclusive and must not precede DTSTART",
                );
            }
        }
    }
    if prop(e, "DTEND").is_some() && prop(e, "DURATION").is_some() {
        issue(
            issues,
            "ERROR",
            "ICS-END-AND-DURATION",
            Some(id.clone()),
            "VEVENT must not contain both DTEND and DURATION",
        );
    }
    if let Some(p) = prop(e, "DURATION") {
        match parse_duration(&p.value) {
            Some(d) if d <= Duration::zero() => issue(
                issues,
                "ERROR",
                "ICS-DURATION-NONPOSITIVE",
                Some(id.clone()),
                "event DURATION must be positive",
            ),
            None => issue(
                issues,
                "ERROR",
                "ICS-DURATION-MALFORMED",
                Some(id.clone()),
                "unsupported or malformed DURATION value",
            ),
            _ => {}
        }
    }
    if let Some(rule) = prop(e, "RRULE") {
        if let Some(start) = prop(e, "DTSTART") {
            if let Ok((Some(dt), _, tz)) = parsed_dt(start) {
                let dtstart = if start.value.ends_with('Z') {
                    format!("DTSTART:{}Z", dt.format("%Y%m%dT%H%M%S"))
                } else if let Some(tz) = tz {
                    format!("DTSTART;TZID={tz}:{}", dt.format("%Y%m%dT%H%M%S"))
                } else {
                    format!("DTSTART:{}", dt.format("%Y%m%dT%H%M%S"))
                };
                if let Err(err) = RRuleSet::from_str(&format!("{dtstart}\nRRULE:{}", rule.value)) {
                    issue(
                        issues,
                        "ERROR",
                        "ICS-RRULE-MALFORMED",
                        Some(id.clone()),
                        err.to_string(),
                    );
                }
            }
        }
        if rule.value.to_ascii_uppercase().contains("COUNT=")
            && rule.value.to_ascii_uppercase().contains("UNTIL=")
        {
            issue(
                issues,
                "ERROR",
                "ICS-RRULE-COUNT-UNTIL",
                Some(id.clone()),
                "RRULE must not contain both COUNT and UNTIL",
            );
        }
    }
    if prop(e, "SUMMARY").is_none() {
        issue(
            issues,
            "INFO",
            "ICS-SUMMARY-ABSENT",
            Some(id),
            "VEVENT has no SUMMARY; this is valid but may be difficult to identify in clients",
        );
    }
}
fn audit_data(path: &Path, events: &[Event]) -> AuditReport {
    let mut issues = Vec::new();
    let mut uids = HashMap::new();
    for (index, event) in events.iter().enumerate() {
        audit_event(event, index + 1, &mut issues, &mut uids);
    }
    for (uid, count) in uids {
        if count > 1 {
            issue(
                &mut issues,
                "ERROR",
                "ICS-UID-DUPLICATE",
                Some(uid.clone()),
                format!("UID appears in {count} VEVENT components"),
            );
        }
    }
    if events.is_empty() {
        issue(
            &mut issues,
            "WARNING",
            "ICS-NO-EVENTS",
            None,
            "calendar contains no VEVENT components",
        );
    }
    let mut counts = BTreeMap::new();
    for i in &issues {
        *counts.entry(i.severity.to_string()).or_insert(0) += 1;
    }
    let result = if issues.iter().any(|i| i.severity == "ERROR") {
        "ERROR"
    } else if issues.iter().any(|i| i.severity == "WARNING") {
        "WARNING"
    } else if issues.iter().any(|i| i.severity == "INFO") {
        "INFO"
    } else {
        "PASS"
    };
    AuditReport {
        schema_version: 1,
        chronogrid_version: env!("CARGO_PKG_VERSION"),
        file: path.display().to_string(),
        scope: "RFC 5545 diagnostic subset; not full conformance",
        result,
        events: events.len(),
        issue_counts: counts,
        issues,
    }
}
fn parse_duration(s: &str) -> Option<Duration> {
    if !s.starts_with('P') {
        return None;
    }
    let (date_part, time_part) = s[1..].split_once('T').unwrap_or((&s[1..], ""));
    if date_part.is_empty() && time_part.is_empty() {
        return None;
    }
    let date_components = parse_components(date_part);
    let time_components = parse_components(time_part);
    if (!date_part.is_empty() && date_components.is_empty())
        || (!time_part.is_empty() && time_components.is_empty())
    {
        return None;
    }
    if date_components.iter().any(|(_, unit)| *unit == 'W') {
        if date_components.len() == 1 && time_part.is_empty() {
            let seconds = i128::from(date_components[0].0) * 7 * 86_400;
            return i64::try_from(seconds).ok().and_then(Duration::try_seconds);
        }
        return None;
    }
    let mut days = 0i64;
    let mut hours = 0i64;
    let mut minutes = 0i64;
    let mut seconds = 0i64;
    let mut seen = HashSet::new();
    for (n, unit) in date_components {
        if unit != 'D' || !seen.insert(unit) {
            return None;
        }
        days = n;
    }
    seen.clear();
    for (n, unit) in time_components {
        if !seen.insert(unit) {
            return None;
        }
        match unit {
            'H' => hours = n,
            'M' => minutes = n,
            'S' => seconds = n,
            _ => return None,
        }
    }
    let seconds = i128::from(days) * 86_400
        + i128::from(hours) * 3_600
        + i128::from(minutes) * 60
        + i128::from(seconds);
    i64::try_from(seconds).ok().and_then(Duration::try_seconds)
}
fn parse_components(s: &str) -> Vec<(i64, char)> {
    let mut n = String::new();
    let mut out = Vec::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            n.push(c);
        } else if let Ok(v) = n.parse() {
            out.push((v, c));
            n.clear();
        } else {
            return Vec::new();
        }
    }
    if !n.is_empty() {
        return Vec::new();
    }
    out
}
fn print_audit(r: &AuditReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(r)?),
        OutputFormat::Text => {
            println!(
                "ICS audit: {} ({})\nScope: {}\nEvents: {}",
                r.file, r.result, r.scope, r.events
            );
            if r.issues.is_empty() {
                println!("No issues found.");
            }
            for i in &r.issues {
                println!("{:<7} {:<24} {}", i.severity, i.code, i.message);
            }
        }
    }
    Ok(())
}
pub(crate) fn audit(path: &Path, format: OutputFormat) -> Result<()> {
    let (_, events) = parse(path)?;
    print_audit(&audit_data(path, &events), format)
}
fn start_as_rrule(p: &Property) -> Result<String> {
    let (Some(dt), _, tz) = parsed_dt(p)? else {
        bail!("DATE DTSTART recurrence expansion is not supported yet");
    };
    if !p.value.ends_with('Z') && tz.is_none() {
        bail!("floating DTSTART cannot be expanded deterministically; provide an explicit TZID or UTC value");
    }
    Ok(if p.value.ends_with('Z') {
        format!("DTSTART:{}Z", dt.format("%Y%m%dT%H%M%S"))
    } else if let Some(tz) = tz {
        format!("DTSTART;TZID={tz}:{}", dt.format("%Y%m%dT%H%M%S"))
    } else {
        format!("DTSTART:{}", dt.format("%Y%m%dT%H%M%S"))
    })
}
pub(crate) fn expand(path: &Path, from: &str, to: &str, format: OutputFormat) -> Result<()> {
    let from = NaiveDate::parse_from_str(from, "%Y-%m-%d").context("--from must be YYYY-MM-DD")?;
    let to = NaiveDate::parse_from_str(to, "%Y-%m-%d").context("--to must be YYYY-MM-DD")?;
    if from > to {
        bail!("--from must not be after --to");
    }
    let (_, events) = parse(path)?;
    let mut out = Vec::new();
    for e in &events {
        let Some(start_prop) = prop(e, "DTSTART") else {
            continue;
        };
        let start_spec = start_as_rrule(start_prop)?;
        let mut lines = vec![start_spec];
        if let Some(rule) = prop(e, "RRULE") {
            lines.push(format!("RRULE:{}", rule.value));
        }
        for key in ["RDATE", "EXDATE"] {
            for p in values(e, key) {
                lines.push(format!("{key}:{}", p.value));
            }
        }
        let set =
            RRuleSet::from_str(&lines.join("\n")).context("invalid recurrence set in VEVENT")?;
        let result = set.all(10_000);
        if result.limited
            && result
                .dates
                .last()
                .is_some_and(|last| last.date_naive() < from)
        {
            bail!("recurrence expansion reached the 10000-instance safety limit before --from; narrow the source rule or date range");
        }
        let exclusions: HashSet<String> = values(e, "EXDATE")
            .iter()
            .map(|p| p.value.clone())
            .collect();
        let end_delta = if let Some(end) = prop(e, "DTEND") {
            let (Some(a), _, _) = parsed_dt(start_prop)? else {
                continue;
            };
            let (Some(b), _, _) = parsed_dt(end)? else {
                continue;
            };
            b - a
        } else if let Some(d) = prop(e, "DURATION").and_then(|p| parse_duration(&p.value)) {
            d
        } else {
            Duration::zero()
        };
        for occurrence in result.dates {
            if occurrence.date_naive() < from || occurrence.date_naive() > to {
                continue;
            }
            let stamp = occurrence.format("%Y%m%dT%H%M%S").to_string();
            if exclusions.contains(&stamp) {
                continue;
            }
            let end = occurrence + end_delta;
            out.push(Occurrence {
                uid: prop(e, "UID").map(|p| p.value.clone()),
                summary: prop(e, "SUMMARY").map(|p| unescape(&p.value)),
                original_start: start_prop.value.clone(),
                occurrence_start: occurrence.to_rfc3339(),
                occurrence_end: if end_delta == Duration::zero() {
                    None
                } else {
                    Some(end.to_rfc3339())
                },
                timezone: start_prop.params.get("TZID").cloned(),
                recurrence_source: if prop(e, "RRULE").is_some() {
                    "RRULE".into()
                } else {
                    "DTSTART".into()
                },
            });
        }
    }
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&out)?),
        OutputFormat::Text => {
            for row in &out {
                println!(
                    "{}  {}  {}",
                    row.uid.as_deref().unwrap_or("(no UID)"),
                    row.occurrence_start,
                    row.summary.as_deref().unwrap_or("(no summary)")
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_supports_basic_positive_units() {
        assert_eq!(parse_duration("P1DT2H"), Some(Duration::hours(26)));
        assert_eq!(parse_duration("P1W"), Some(Duration::days(7)));
        assert_eq!(parse_duration("P0D"), Some(Duration::zero()));
        assert_eq!(parse_duration("P"), None);
    }
    #[test]
    fn unfolded_content_line_is_joined() {
        let path = std::env::temp_dir().join(format!("chronogrid-{}.ics", std::process::id()));
        std::fs::write(&path,"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:one\r\nDTSTAMP:20270101T000000Z\r\nDTSTART:20270102T090000Z\r\nSUMMARY:folded\r\n line\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n").unwrap();
        let (_, events) = parse(&path).unwrap();
        assert_eq!(prop(&events[0], "SUMMARY").unwrap().value, "foldedline");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn audit_flags_gap_and_duplicate_uid() {
        let mut events = Vec::new();
        for _ in 0..2 {
            events.push(vec![
                Property {
                    name: "UID".into(),
                    params: HashMap::new(),
                    value: "duplicate".into(),
                },
                Property {
                    name: "DTSTAMP".into(),
                    params: HashMap::new(),
                    value: "20270101T000000Z".into(),
                },
                Property {
                    name: "DTSTART".into(),
                    params: HashMap::from([("TZID".into(), "America/New_York".into())]),
                    value: "20270314T023000".into(),
                },
            ]);
        }
        let report = audit_data(Path::new("calendar.ics"), &events);
        assert_eq!(report.result, "ERROR");
        assert!(report.issues.iter().any(|i| i.code == "ICS-DST-GAP"));
        assert!(report.issues.iter().any(|i| i.code == "ICS-UID-DUPLICATE"));
    }
}
