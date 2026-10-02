# Timezone semantics

ChronoGrid uses `chrono-tz`'s compiled IANA timezone data. A local wall time may map to one UTC instant, two instants (a fold), or none (a gap). `local classify` reports all applicable candidates. `event matrix` requires `--fold earlier|later` for an ambiguous source and rejects a nonexistent one.

RRULE DTSTART follows the same conservative rule: ambiguous and nonexistent starts are errors. The recurrence library handles later recurrence instances according to its RFC implementation. The `doctor` command reports the timezone engine but not a tzdb release number because the selected API does not expose one.
