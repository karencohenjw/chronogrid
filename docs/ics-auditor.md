# ICS auditor scope

The auditor unfolds RFC-style folded content lines and recognizes VCALENDAR, VEVENT, UID, DTSTAMP, DTSTART, DTEND, DURATION, TZID, RRULE, RDATE, EXDATE, and SUMMARY. It checks required event fields, duplicates, date-time parsing, TZID validity, floating local starts, DST gaps/folds, date value consistency, ordering, and basic positive durations.

This is a diagnostic subset, not a complete RFC 5545 parser. It does not implement VTIMEZONE observances, alarms, attendee semantics, recurrence overrides, every property value type, or all escaping/encoding rules. Unknown properties are retained only as uninterpreted content and are not validated. ICS expansion currently supports date-time DTSTART and bounded RRULE/RDATE/EXDATE iteration; all-day expansion is not supported.

Severity means: PASS (no diagnostics), INFO (context only), WARNING (potential interoperability issue), ERROR (invalid or contradictory supported data).
