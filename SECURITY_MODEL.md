# MedTracker security model

Scope: `damacus/med-tracker` at `main` (Rust / Loco 1.2 application, PostgreSQL 18), with `damacus/home-ops` as the deployment reference. Built from the code and existing evidence: `docs/rust-security-review.md`, `docs/security/hosted-multi-tenant-hardening-audit.md` (Rails-era launch gate), `docs/design.md`, `AGENTS.md`.

## System overview

MedTracker is a self-hosted medication tracker for families and carers: household medication schedules, dose recording ("takes"), stock, reminders, reports and a tamper-evident audit trail. **User-confirmed: both deployment models matter**: a self-hosted instance with one family household, and a hosted multi-tenant instance where many independent households share one app and one database.

Deployment facts (home-ops): `kubernetes/apps/home/med-tracker` serves `med-tracker.damacus.io` (external-dns target `traefik-ext.damacus.io`, so internet-reachable) and `med-tracker.ironstone.casa` (internal) through the Traefik Gateway. The NetworkPolicy admits ingress only from Traefik pods on port 80, and TLS ends at Traefik. Secrets come from 1Password through ExternalSecrets (`med-tracker-secret`), the database is CloudNativePG (`med-tracker-db`, with backups to R2), and OIDC goes to Zitadel (`zitadel.damacus.io`). Production still runs the Rails image 0.5.35. The Rust image (`ghcr.io/damacus/med-tracker-loco`) runs only in `kubernetes/apps/medtracker-canary` (internal gateway). AGENTS.md says no production deployment of the Rust app is authorised yet. Findings in the Rust code are therefore pre-production, but they will ship to the internet-facing host.

## Components

| Component | Covers |
| --- | --- |
| **Identity & Auth** | `src/models/identity/**` (Better Auth: passwords, TOTP, passkeys/WebAuthn, OIDC, sessions, personal keys, signup/bootstrap, rate limits), `src/controllers/auth.rs`, `src/controllers/identity_onboarding*`, `src/controllers/oauth_server/**` (`/token`, `/revoke`, consent), `src/controllers/invitations*`, `src/controllers/api/account_sessions*`, `src/models/rate_limit.rs`, `src/controllers/api/rate_limit.rs` |
| **Care API** | `src/controllers/api/care*/**` (`/api/v1` household-scoped REST: medications, doses, sync, portable imports, data exports, sharing, AI suggestions, push/device tokens) |
| **Web UI** | browser controllers in `src/controllers/*` (dashboard, medications, people, profile/avatar, treatments, reports, searches, dose occurrences, locations, guardian declarations), `src/views/**` (Tera), `assets/**` (JS, service worker) |
| **Care Domain & Authorization** | `src/models/care/**`, `src/models/access/**`, `src/models/authorization/**` (Cedar policies `policies.cedar`, `matrix.rs`), `src/models/profile/**`, `src/models/notification_preferences/**` |
| **Tenancy & Data** | `src/models/household_tenancy.rs`, `src/models/identity/store.rs` and `browser.rs` (`SET LOCAL ROLE med_tracker_app`, `set_config('med_tracker.current_*')` RLS context), `src/models/_entities/**`, `src/models/audit_context.rs`, `migration/**` (RLS policies, runtime role provisioning), `src/app.rs` |
| **Platform Admin** | `src/controllers/platform*`, `src/controllers/administration*`, `src/models/platform/**` (support access, recovery, settings, database setup) |
| **Workers & Integrations** | `src/models/nhs_dmd/**`, `src/models/care/review_prompts/refresh.rs` (openFDA/DailyMed), browser push and mail outbox, `src/models/care/ai_suggestions.rs` (OpenAI/Anthropic/Gemini/OpenRouter), `src/models/profile/gravatar.rs`, `src/models/storage.rs` (S3/RustFS), `src/models/canary/**`, `src/telemetry/**` |
| **Client Tools & Mobile** | `client-tools/**` (API client, CLI, MCP server using `MEDTRACKER_TOKEN`), `mobile/android/**` |
| **Deployment** | `Dockerfile`, `deploy/helm/**`, `compose*.yaml`, `config/*.yaml`, `.github/workflows/**`, `Taskfiles/**`, plus home-ops manifests |

## Trust boundaries and entry points

1. **Internet → Traefik → app** (anonymous): `/up`, `/health`, the Better Auth router (`before_routes`), login, signup and passkey ceremonies, OIDC callback, invitation acceptance, `/.well-known/oauth-authorization-server`, `/token`, `/revoke`, static assets.
2. **Authenticated browser session**: every `browser(...)` route in `src/app.rs`, with CSRF and cookie sessions. The household is selected through `/households/select`.
3. **Bearer-token API** (`/api/v1/households/{id}/...`): first-party mobile/CLI/MCP OAuth tokens and app tokens, which are membership-bound and scoped. The household id in the path is attacker-controlled and must be authorized against live membership.
4. **App → PostgreSQL**: the runtime role `med_tracker_app` (NOBYPASSRLS). Tenant isolation relies on the transaction-local `med_tracker.current_account_id`, `current_household_id`, `current_membership_id` and `current_invitation_token_digest`, plus forced RLS.
5. **App → third parties**: LLM providers (household medication data leaves the system), openFDA/NHS dm+d, Gravatar, web push endpoints (user-supplied URLs), SMTP, S3.
6. **Platform admin / support access**: an in-app operator capability, separate from household membership.

## Assets (user-ranked)

- **Critical (user-confirmed):** health and medication records: people, medications, schedules, dose history, health events, reports, and exports (special-category health data under UK GDPR).
- **Critical (user-confirmed):** account credentials, sessions, passkeys/WebAuthn credentials, TOTP secrets, OAuth/API tokens, invitation tokens.
- Other assets (inferred from the code, not ranked by the user): integrity of dose recording and stock, the tamper-evident audit trail, portable encrypted export/import payloads, and app secrets (`RAILS_SECRET_KEY_BASE`, OIDC client secret, web-push private key, S3 keys, LLM API keys).

## Attackers that matter (user-confirmed)

- **Anonymous internet user**: the external hostname is public.
- **Compromised API/CLI/MCP client or stolen token.**
- **User in another household**: cross-tenant access on the shared hosted database.
- **Malicious or curious member/carer in the same household**: cross-person access beyond their `PersonAccessGrant` (view/record/manage), or escalation of their household role (member → administrator → owner).

## Out of scope / accepted risk (user-confirmed)

- **Platform admins are fully trusted.** A finding that requires a platform admin to act maliciously, or that only lets a platform admin do something, is accepted risk. Findings where a *non-admin* reaches platform-admin capability are still in scope and are critical.

Nothing else was ruled out: mobile, client tools, CI and DoS all stay in scope at the scanner's severity.

## Existing controls (from code and docs)

- Forced RLS with default-deny, a separate migrator role and a `NOBYPASSRLS` runtime role. Tenant context is set transaction-locally (`store.rs`, `browser.rs`). Production boot calls `validate_runtime`.
- Cedar authorization policies (`src/models/authorization/policies.cedar`) and live membership/permission checks on each request. Permission versions revoke stale credentials.
- WebAuthn via `webauthn-rs` 0.5.5 with challenge-replay protection (unique digest, ten-minute lifetime). Auth rate limits are configured through `MEDTRACKER_AUTH_LIMIT_*` and `API_TRUSTED_PROXY_IPS`.
- Idempotency keys and version preconditions on API writes, atomic stock deduction, a mail outbox, and an append-only audit chain.
- A scratch/non-root production container. In home-ops, the NetworkPolicy allows ingress from Traefik only.

## Open questions

- Is the external route meant to stay on the `traefik-internal` gateway, with only the DNS target pointing at `traefik-ext`? This affects which proxy IPs `API_TRUSTED_PROXY_IPS` should trust.
- When hosted multi-tenant, is AI suggestions (data sent to third-party LLMs) enabled per household, and who supplies the API keys?
