# ChronoGrid

A deterministic date, calendar, and timezone conformance CLI for reproducing temporal edge cases.

## Features

- Gregorian date inspection with ISO week, ordinal day, leap-year, and IANA timezone details
- ISO week inspection and Monday- or Sunday-start month grids
- Gregorian ordinal, ISO week, Julian Date (JD), Julian Day Number (JDN), and Modified Julian Date (MJD) conversions
- IANA timezone offset transition reports sourced from the bundled chrono-tz database
- Deterministic conformance vectors and seeded edge-case generation
- Text and JSON output; optional HTML challenge report

## Examples

- `chronogrid date inspect 2027-01-01 --timezone America/New_York`
- `chronogrid date inspect 2027-01-01 --timezone America/New_York --format json`
- `chronogrid week inspect 2027-01-01`
- `chronogrid calendar month 2027 1 --week-start monday`
- `chronogrid calendar month 2027 1 --week-start sunday --format json`
- `chronogrid convert gregorian 2000-01-01`
- `chronogrid timezone inspect America/New_York --year 2027`
- `chronogrid challenge --format json --report report.html`
- `chronogrid chaos --from 1900 --to 2100 --seed chronogrid-demo --count 100 --format json`

Run tests with `cargo test --all-features`; validate formatting and lints with `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.

## Scope

This release supports the Gregorian civil calendar, ISO week dates, Julian date conventions, and IANA timezone transitions. ICS parsing and RRULE recurrence expansion are not implemented in v0.1.0.

## License

See [LICENSE](LICENSE) if present in this repository.
