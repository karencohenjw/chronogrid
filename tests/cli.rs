use serde_json::Value;
use std::process::{Command, Output};

fn chronogrid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chronogrid"))
        .args(args)
        .output()
        .expect("chronogrid process starts")
}
fn json(args: &[&str]) -> Value {
    let output = chronogrid(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

#[test]
fn legacy_date_and_timezone_commands_still_work() {
    let date = json(&[
        "date",
        "inspect",
        "2027-01-01",
        "--timezone",
        "America/New_York",
        "--format",
        "json",
    ]);
    assert_eq!(date["iso_week_year"], 2026);
    assert_eq!(date["iso_week"], 53);
    let zones = json(&[
        "timezone",
        "inspect",
        "America/New_York",
        "--year",
        "2027",
        "--format",
        "json",
    ]);
    assert_eq!(zones["transitions"].as_array().unwrap().len(), 2);
}

#[test]
fn local_classification_covers_fold_and_gap() {
    let fold = json(&[
        "local",
        "classify",
        "2027-11-07T01:30:00",
        "--timezone",
        "America/New_York",
        "--format",
        "json",
    ]);
    assert_eq!(fold["classification"], "ambiguous");
    assert_eq!(fold["candidates"].as_array().unwrap().len(), 2);
    let gap = json(&[
        "local",
        "classify",
        "2027-03-14T02:30:00",
        "--timezone",
        "America/New_York",
        "--format",
        "json",
    ]);
    assert_eq!(gap["classification"], "nonexistent");
    assert!(gap["transition"]
        .as_str()
        .unwrap()
        .contains("jumps forward"));
}

#[test]
fn event_matrix_requires_explicit_fold_and_renders_offsets() {
    let base = [
        "event",
        "matrix",
        "--datetime",
        "2027-11-07T01:30:00",
        "--timezone",
        "America/New_York",
        "--to",
        "Europe/Istanbul",
    ];
    assert!(!chronogrid(&base).status.success());
    let report = json(&[
        "event",
        "matrix",
        "--datetime",
        "2027-11-07T01:30:00",
        "--timezone",
        "America/New_York",
        "--to",
        "Europe/Istanbul",
        "--fold",
        "later",
        "--format",
        "json",
    ]);
    assert_eq!(report["rows"][0]["utc_offset"], "+03:00");
    assert_eq!(report["rows"][0]["date_shift_days"], 0);
}

#[test]
fn rrule_expansion_honors_count_and_monthly_bysetpos() {
    let report = json(&[
        "rrule",
        "expand",
        "--start",
        "2027-01-04T09:00:00",
        "--timezone",
        "Europe/Istanbul",
        "--rule",
        "FREQ=MONTHLY;BYDAY=MO;BYSETPOS=1;COUNT=12",
        "--format",
        "json",
    ]);
    let dates = report["occurrences"].as_array().unwrap();
    assert_eq!(dates.len(), 12);
    assert_eq!(dates[0], "2027-01-04T09:00:00+03:00");
    assert_eq!(dates[1], "2027-02-01T09:00:00+03:00");
    assert_eq!(report["truncated"], false);
}

#[test]
fn committed_ics_fixture_audits_and_expands_exdate() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/weekly-event.ics");
    let audit = chronogrid(&["ics", "audit", file, "--format", "json"]);
    assert!(
        audit.status.success(),
        "{}",
        String::from_utf8_lossy(&audit.stderr)
    );
    let report: Value = serde_json::from_slice(&audit.stdout).unwrap();
    assert_eq!(report["result"], "PASS");
    assert_eq!(report["events"], 1);
    let expanded = chronogrid(&[
        "ics",
        "expand",
        file,
        "--from",
        "2027-01-01",
        "--to",
        "2027-01-31",
        "--format",
        "json",
    ]);
    assert!(
        expanded.status.success(),
        "{}",
        String::from_utf8_lossy(&expanded.stderr)
    );
    let occurrences: Value = serde_json::from_slice(&expanded.stdout).unwrap();
    assert_eq!(occurrences.as_array().unwrap().len(), 3);
    assert!(occurrences
        .as_array()
        .unwrap()
        .iter()
        .all(|o| !o["occurrence_start"]
            .as_str()
            .unwrap()
            .starts_with("2027-01-18")));
}

#[test]
fn challenge_outputs_at_least_fifty_passing_vectors() {
    let report = json(&["challenge", "--format", "json"]);
    assert!(report["vectors_executed"].as_u64().unwrap() >= 50);
    assert_eq!(report["failed"], 0);
    assert_eq!(report["passed"], report["vectors_executed"]);
}

#[test]
fn year_calendar_and_doctor_are_machine_readable() {
    let year = json(&[
        "calendar",
        "year",
        "2027",
        "--week-start",
        "sunday",
        "--format",
        "json",
    ]);
    assert_eq!(year["months"].as_array().unwrap().len(), 12);
    let doctor = json(&["doctor", "--format", "json"]);
    assert_eq!(doctor["schema_version"], 1);
    assert_eq!(doctor["timezone_database_version"], Value::Null);
}
