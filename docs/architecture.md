# Architecture

ChronoGrid keeps the command-line adapter in `legacy.rs` while the executable entry point in `main.rs` only wires feature modules. `temporal.rs` owns local wall-time classification, instant conversion, and host diagnostics. `recurrence.rs` adapts bounded RRULE expansion. `ical.rs` handles content-line unfolding, the supported VEVENT subset, diagnostics, and recurrence set expansion. Existing Gregorian, calendar-grid, challenge, and seeded-chaos behavior remains in the CLI module pending further extraction.

All calendar input is parsed into explicit chrono types before calculations. IANA zone names are validated by `chrono-tz`; no OS-local timezone assumption is used in timezone-aware commands. New JSON reports use schema version 1.
