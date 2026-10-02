# RRULE support

The CLI delegates recurrence parsing and generation to `rrule` 0.14, a Rust implementation of RFC 5545 recurrence rules under MIT or Apache-2.0. The dependency supports RFC recurrence sets, including DTSTART, RRULE, RDATE, and EXDATE. ChronoGrid caps one expansion request at 10,000 returned CLI instances and refuses ambiguous or nonexistent local DTSTART values.

The dependency supports DAILY, WEEKLY, MONTHLY, and YEARLY as well as RFC rule parts, including INTERVAL, COUNT, UNTIL, BYDAY, BYMONTH, BYMONTHDAY, BYSETPOS, and WKST. Unsupported or invalid parts are rejected by the parser. RRULE behavior around non-existent recurrence wall times follows the dependency's RFC interpretation and should be checked against the linked crate documentation for interoperability-sensitive applications.

ChronoGrid does not claim that its ICS parser fully supports recurrence overrides or all RFC 5545 recurrence edge cases.
