# Console billing fixtures

## Provenance — read this before trusting these files

These fixtures are **derived from a third-party implementation, not captured from a live
response.** The source is [hamed-elfayome/Claude-Usage-Tracker][src] (MIT), a macOS menu-bar app
whose `ClaudeAPIService+ConsoleAPI.swift` calls these endpoints in production. The field names,
types, and nesting below are transcribed from its `Codable` structs.

That is weaker evidence than this project's own policy asks for. It is stronger than a guess:
the keys are not invented here, they are read off a working client with a large user base. The
distinction that matters — **no response body from a real account has been inspected by this
project.** Values in these files are synthetic.

Consequently:

- Parsers may be written against these shapes, because a wrong key fails loudly (a missing field
  is `None`, never a silent zero).
- **Optional fields stay optional.** Anything the source marks nullable is nullable here.
- Amounts are never trusted for scale beyond what the source names: `current_spend.amount` and
  `prepaid/credits.amount` are integer **cents**; `usage_cost[].total` is a fractional **cent**
  value serialized as a JSON number.

To upgrade this to a real fixture: sign in at console.anthropic.com, capture the response, redact
identifiers only (keep original keys), and record endpoint, method, status, credential type,
required role, and capture date here.

## Live probe — 2026-08-30

Unauthenticated GETs were sent to every endpoint below (no credentials, no data returned) to
establish which routes exist. Results:

- **`console.anthropic.com` is gone.** It answers `301 Moved Permanently` →
  `https://platform.claude.com`. The upstream Swift client still uses the old host and relies on
  its HTTP stack following the redirect. **This client must not**: it runs
  `redirect(Policy::none())`, so it targets `platform.claude.com` directly.
- Four routes returned `403 permission_error` / `account_session_invalid` — they exist and are
  session-gated. See `unauthorized-403.json`, which **is** a real captured body.
- Two control paths (`/totally_bogus_root`, `/{org}/this_is_not_an_endpoint`) returned
  `404 not_found_error`, so the 403s above are meaningful and not a blanket response.
- **`/{org}/api_keys` returned `404`, identical to the controls.** The other four return 403 with
  the same fake org UUID, so auth is checked before org existence; a 404 here points at the route
  being gone rather than the org being fake. Treated as unavailable: cost-by-key falls back to the
  redacted key id, and no replacement path is guessed.

| Route | Status (unauth) | Verdict |
|---|---|---|
| `/organizations` | 403 | exists |
| `/organizations/{uuid}/current_spend` | 403 | exists |
| `/organizations/{uuid}/prepaid/credits` | 403 | exists |
| `/organizations/{uuid}/workspaces/default/usage_cost` | 403 | exists |
| `/organizations/{uuid}/api_keys` | 404 | **unverified — capability off** |
| `/totally_bogus_root` (control) | 404 | — |

Existence is proven; **response shape is not.** A 403 says nothing about what a 200 looks like.
The success-path shapes below remain third-party-derived until someone captures a real 200.

## Endpoints

Base: `https://platform.claude.com/api` — the Console **web** backend (formerly
`console.anthropic.com`), not the documented `api.anthropic.com` platform API. Undocumented and
unversioned; it can change without notice.

Auth: `Cookie: sessionKey=<console.anthropic.com session cookie>`. This is **not** an `sk-ant-`
API key and **not** the claude.ai session key — it is a third, separate credential.

| File | Endpoint | Notes |
|---|---|---|
| `organizations.json` | `GET /organizations` | `uuid` is the org id used by every other call |
| `current-spend.json` | `GET /organizations/{uuid}/current_spend` | integer cents |
| `prepaid-credits.json` | `GET /organizations/{uuid}/prepaid/credits` | integer cents + ISO 4217 |
| `usage-cost.json` | `GET /organizations/{uuid}/workspaces/default/usage_cost` | `?starting_on=&ending_before=&group_by=api_key_id`; keyed by date |
| `api-keys.json` | `GET /organizations/{uuid}/api_keys?status=active` | id → display name |
| `partial-permissions.json` | any of the above | synthetic 403 body |
| `unauthorized-403.json` | any of the above | **real captured 403**, request id redacted |

`usage_cost` returns three sibling maps — `costs`, `web_search_costs`, `code_execution_costs` —
each `{ "YYYY-MM-DD": [entry, ...] }`. All three are optional and are summed together.

[src]: https://github.com/hamed-elfayome/Claude-Usage-Tracker
