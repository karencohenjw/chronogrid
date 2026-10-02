# ChronoGrid

ChronoGrid is a deterministic temporal engineering workbench for testing calendar boundaries, time zones, recurrence rules, and ICS files. It is intended for developers and test engineers building scheduling, booking, billing, and calendar software.

Dates fail at the seams: DST creates missing and repeated local times, time-zone offsets cross date boundaries, ISO weeks do not follow Gregorian year boundaries, and recurrence rules have precise expansion semantics. ChronoGrid makes those cases inspectable from a scriptable CLI.

## Installation

### Chocolatey

After the community package is approved, install it with:

```powershell
choco install chronogrid
```

### Build from source

Requires stable Rust:

```sh
cargo install --git https://github.com/karencohenjw/chronogrid --tag v0.2.0
```

Windows release archives are attached to the [v0.2.0 release](https://github.com/karencohenjw/chronogrid/releases/tag/v0.2.0) when published.

## Command overview

| Command | Purpose |
| --- | --- |
| `date inspect` | Gregorian, ordinal, ISO week, and local-midnight facts |
| `week inspect` | ISO week-year inspection |
| `calendar month` | Structured Monday- or Sunday-first month grid |
| `convert gregorian` | Ordinal, ISO, JD, JDN, and MJD conversions |
| `timezone inspect` | IANA offset transitions for a year |
| `local classify` | Detect a unique time, DST fold, or DST gap |
| `event matrix` | Render one selected instant in several zones |
| `rrule expand` | Bounded RFC recurrence expansion |
| `ics audit` | Diagnostics for a documented RFC 5545 subset |
| `ics expand` | Expand supported date-time events in a date range |
| `challenge` | Deterministic temporal conformance vectors |
| `chaos` | Seeded boundary-focused case generation |
| `doctor` | Runtime and build diagnostics |

Use `chronogrid <command> --help` for command-specific options. Commands that accept `--format json` produce machine-readable output.

## Local time diagnostics

```sh
chronogrid local classify 2027-11-07T01:30:00 --timezone America/New_York
chronogrid local classify 2027-03-14T02:30:00 --timezone America/New_York --format json
```

Results distinguish unique times, fall-back folds, and spring-forward gaps. Folds list both UTC instants and offsets. Gaps report the adjacent clock transition. ChronoGrid does not silently choose an instant for a fold.

## Timezone event matrix

```sh
chronogrid event matrix --datetime 2027-03-28T00:30:00 \
  --timezone Europe/London \
  --to America/New_York,Europe/Istanbul,Asia/Tokyo,Australia/Sydney
```

The command resolves the source local time first, then shows the same UTC instant in each target zone with offsets and date-shift information. For an ambiguous source, specify `--fold earlier` or `--fold later`; nonexistent source times are rejected.

## ICS audit

```sh
chronogrid ics audit calendar.ics --format json
```

The auditor reads unfolded VCALENDAR/VEVENT content lines and checks event identifiers, timestamps, date type consistency, local-time anomalies, durations, duplicate UIDs, and recurrence-rule syntax. Its scope and unsupported cases are reported; this is not a full RFC 5545 validator. See [ICS auditor scope](docs/ics-auditor.md).

## Recurrence expansion

```sh
chronogrid rrule expand --start 2027-01-04T09:00:00 \
  --timezone Europe/Istanbul \
  --rule 'FREQ=MONTHLY;BYDAY=MO;BYSETPOS=1;COUNT=12' --format json
```

ChronoGrid uses the `rrule` Rust crate for RFC recurrence semantics and caps requested output at 10,000 instances. An ambiguous or nonexistent DTSTART is refused. Supported rule parts and limitations are documented in [RRULE support](docs/rrule-support.md).

ICS expansion accepts date-time DTSTART events and applies RRULE, RDATE, and EXDATE sets within a requested date range:

```sh
chronogrid ics expand calendar.ics --from 2027-01-01 --to 2027-12-31
```

## Calendar conformance and deterministic chaos

```sh
chronogrid challenge --format json
chronogrid chaos --from 1900 --to 2400 --seed release-check --count 100 --format json
```

Chaos vectors are generated locally from a stable seed; no network or random service is involved. `challenge` checks named Gregorian, ISO-week, timezone, and calendar-grid invariants.

## JSON and diagnostics

JSON responses use stable snake_case keys. New structured reports include `schema_version` and `chronogrid_version`. Error messages go to stderr with a non-zero exit status. `chronogrid doctor --format json` reports the OS and architecture and identifies the embedded timezone engine; it does not invent a tzdb version.

## Security and offline behavior

The executable performs no telemetry or hidden network requests. CLI calculations operate locally. ICS files are read from the path supplied by the user. The Chocolatey installer downloads only immutable versioned release archives and verifies their SHA-256 checksums.

## Standards and scope

ChronoGrid uses IANA zones as compiled into `chrono-tz`. Its ICS auditor implements a useful diagnostic subset, not complete RFC 5545 parsing or validation. RRULE expansion delegates recurrence semantics to the `rrule` crate; supported frequency parts are listed in the dedicated support note.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

See [architecture](docs/architecture.md), [timezone semantics](docs/timezone-semantics.md), [conformance vectors](docs/conformance-vectors.md), and [Chocolatey packaging](docs/chocolatey.md).

## Project and license

ChronoGrid is maintained by Karen Cohen and is part of the JW Calendar project at [jwcalendar.com](https://jwcalendar.com/). The source is licensed under MIT; see [LICENSE](LICENSE).
