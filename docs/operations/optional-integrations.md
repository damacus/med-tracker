# Optional Integration Configuration

MedTracker can call external services for medication search, public guidance,
AI-assisted form suggestions, and browser identity. Each integration is
optional. An absent credential must disable or limit that feature without
blocking normal medication records.

Review the provider's terms, data location, retention, and access controls
before enabling a service. Store every API key in the deployment secret store.

## NHS dm+d terminology search

Live UK terminology search requires both values:

| Variable | Purpose |
| --- | --- |
| `NHS_DMD_CLIENT_ID` | NHS England Terminology Server client ID. |
| `NHS_DMD_CLIENT_SECRET` | NHS England Terminology Server client secret. |

When either value is absent, live NHS terminology search is unavailable. Local
catalogues, imported dm+d data, curated results, and eligible Open Food Facts
lookups can still work.

See [NHS dm+d and medication lookup](../nhs-dmd-integration.md) for account and
import setup.

## NHS website medicine guidance

Medication detail pages can load public NHS medicine guidance.

| Variable | Purpose |
| --- | --- |
| `NHS_WEBSITE_CONTENT_API_KEY` | Subscription key for the NHS Website Content API. |

Without the key, MedTracker records a `not_configured` lookup outcome and shows
no remote guidance. Successful responses are cached for twelve hours.

The lookup sends a normalized medication name to the NHS service. Confirm that
this use matches the deployment's privacy and external-service policy.

## Open Food Facts

Open Food Facts does not require an API key. MedTracker identifies its client
through these optional values:

| Variable | Default | Purpose |
| --- | --- | --- |
| `OPEN_FOOD_FACTS_APP_NAME` | `MedTracker` | Application name in the HTTP user agent. |
| `OPEN_FOOD_FACTS_APP_VERSION` | `1.0` | Deployed application version in the HTTP user agent. |
| `OPEN_FOOD_FACTS_CONTACT_EMAIL` | `support@medtracker.app` | Operator contact in the HTTP user agent. |

Self-hosters should replace the contact email with a monitored address. Barcode
and supplement searches can send the entered query to Open Food Facts even when
NHS dm+d credentials are absent.

## AI medication suggestions

AI medication help is disabled when no supported provider key is present.
MedTracker's initializer supports these provider settings:

| Variable | Provider |
| --- | --- |
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic |
| `GEMINI_API_KEY` | Google Gemini |
| `OPENROUTER_API_KEY` | OpenRouter |

Set one provider key and choose a compatible model:

| Variable | Default | Purpose |
| --- | --- | --- |
| `MEDTRACKER_AI_MEDICATION_HELP_MODEL` | locked RubyLLM default | Model identifier passed to `RubyLLM.chat`. |

Set the model explicitly in production so a RubyLLM default change cannot move
the workload to another model. The chosen model must belong to the configured
provider.

Azure is not a documented provider yet. Issue
[#1897](https://github.com/damacus/med-tracker/issues/1897) tracks the mismatch
between its readiness check and initializer settings.

### Data sent to the model

The assistant receives the medication identity entered for the suggestion. It
can also receive text fetched from URLs in
`config/ai_medication_sources.yml`. Those URLs are restricted to configured
HTTPS domains and paths.

The request contract supplies medication identity rather than person, household,
account, or dose-history data. Treat user-entered text as potentially sensitive.
Do not enable the feature until the selected provider is approved to process
the medication identity and public source text.

MedTracker stores a hash of the requested medication identity with bounded
result counts for audit. It does not store the prompt or model response in that
audit record.

## OpenID Connect

Browser OIDC settings are documented in
[OpenID Connect setup](../oidc-setup.md). The mobile PKCE contract is incomplete
and tracked by issue [#1889](https://github.com/damacus/med-tracker/issues/1889).

## Verification

Verify one integration at a time with synthetic medication data.

1. Add the credential through the deployment secret store.
2. Restart the processes that use it.
3. Exercise one bounded request from a test household.
4. Confirm the response source and expected fallback behavior.
5. Review logs and audit evidence for secret or health-data leakage.
6. Remove test records and rotate any temporary credential.
