# Security Policy

Report security issues privately to the repository owner through GitHub's security advisory feature. Do not include credentials, API keys, or private user data in public issue reports.

ChronoGrid does not make network requests during normal CLI use. The Chocolatey package downloads release archives over HTTPS and validates fixed SHA-256 values. ICS input is untrusted data and should be treated as such; the parser is intentionally scoped and should not be used as a security boundary.
