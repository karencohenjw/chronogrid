# Conformance vectors

`chronogrid challenge` emits deterministic named invariants. Existing vectors cover Gregorian leap rules, ISO week-year boundaries, JD/JDN anchors, month-grid completeness, timezone offset transitions, and non-whole-hour offsets. The JSON output is suitable for smoke checks; it is not a replacement for a full calendar or tzdb conformance suite.

Seeded `chronogrid chaos` samples boundary-oriented cases reproducibly from the seed and does not call external services.
