# Console billing fixtures

No Console web billing or prepaid-balance endpoint is enabled in this build. Anthropic's
documented organization cost-report contract must be captured and reviewed with original keys,
endpoint, method, API version/beta headers, credential type, required role, pagination semantics,
status, capture date, and redactions before enabling a capability. The adapter therefore reports
all sections as `unsupportedBySource`; it never guesses private Console endpoints or derives a
balance from spend.
