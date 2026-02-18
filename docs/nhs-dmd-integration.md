# NHS dm+d Medicine Search Integration

MedTracker integrates with the NHS Dictionary of Medicines and
Devices (dm+d) via the NHS England Terminology Server to let
clinicians search for medicines by name or active ingredient.

## What is dm+d?

The dm+d is the NHS standard catalogue of medicines used across
the UK. It assigns every medicine a unique SNOMED CT code and is
the authoritative source for medicine names in NHS systems.

MedTracker queries two dm+d concept types:

| Type    | Meaning                             |
|---------|-------------------------------------|
| **VMP** | Virtual Medicinal Product (generic) |
| **AMP** | Actual Medicinal Product (branded)  |

Example VMP: `Aspirin 300mg tablets`
Example AMP: `Aspirin 300mg tablets (Bayer)`

## What data comes back?

Each search result contains four fields:

| Field           | Example                      |
|-----------------|------------------------------|
| `code`          | `39720311000001101` (SNOMED) |
| `display`       | `Aspirin 300mg tablets`      |
| `system`        | `https://dmd.nhs.uk`         |
| `concept_class` | `VMP` or `AMP`               |

The dm+d API does **not** include dosage guidance,
contraindications, or drug interaction data.
See [Drug interactions](#drug-interactions) below.

## Getting credentials

Access to the dm+d API requires a **system-to-system account**
from the NHS England Terminology Server. This is separate from
the NHS API Platform (`digital.nhs.uk/developer`).

### Step 1 — Read the account agreement

Read the system-to-system account agreement before applying:
<https://digital.nhs.uk/services/terminology-server/system-to-system-account-agreement>

### Step 2 — Complete the request form

Open the request form and fill it in:
<https://digital.nhs.uk/services/terminology-server/request-a-system-to-system-account/request-form>

When asked for your **purpose**, select **Consumer** —
read-only access to published content. This is the correct
category for an application that searches dm+d at runtime.

You will need to provide:

- Organisation name and type
- Description of your system and its clinical use
- Confirmation you have read the account agreement

### Step 3 — Receive your credentials

NHS England will issue an OAuth2 `client_id` and
`client_secret` for the Analytic Production Server
(`ontology.nhs.uk/production1/fhir`).

> The service is **free of charge** for health and care
> organisations. Approval typically takes a few working days.
> Contact `information.standards@nhs.net` with questions.

### Optional — interactive browser access

You can browse dm+d content interactively without a
system-to-system account by logging in with NHS.net,
Microsoft, GitHub, LinkedIn, or Google at:
<https://ontology.nhs.uk>

This is useful for exploring what the API returns before
writing code.

## Environment variables

Both variables must be set to enable the feature:

| Variable                | Description                       |
|-------------------------|-----------------------------------|
| `NHS_DMD_CLIENT_ID`     | OAuth2 client ID from NHS England |
| `NHS_DMD_CLIENT_SECRET` | OAuth2 client secret              |

If either variable is absent the search feature is
**automatically disabled** — no API calls are made and the
UI shows an amber warning to administrators.

### Local development (Fish shell)

Export the variables in your shell before starting the server:

```fish
set -x NHS_DMD_CLIENT_ID "your-client-id"
set -x NHS_DMD_CLIENT_SECRET "your-client-secret"
task dev:up
```

### Docker Compose

Docker Compose inherits exported shell variables automatically,
so exporting them before `task dev:up` is sufficient. You can
also add them to a `.env` file in the project root (never
commit this file):

```sh
NHS_DMD_CLIENT_ID=your-client-id
NHS_DMD_CLIENT_SECRET=your-client-secret
```

### Production / Kubernetes

Store credentials as a Kubernetes Secret:

```yaml
env:
  - name: NHS_DMD_CLIENT_ID
    valueFrom:
      secretKeyRef:
        name: med-tracker-nhs-dmd
        key: client-id
  - name: NHS_DMD_CLIENT_SECRET
    valueFrom:
      secretKeyRef:
        name: med-tracker-nhs-dmd
        key: client-secret
```

## Feature gating

The search feature is **off by default** when credentials are
not configured. The UI shows:

> *Medicine search not available — NHS dm+d credentials are
> not configured.*

Once both environment variables are present the feature
activates automatically on the next boot — no code change or
restart flag required.

## How the integration works

```text
Browser
  → MedicinesController#search
    → NhsDmd::Search
      → NhsDmd::Client
          POST /token  (OAuth2 client_credentials grant)
          GET  /ValueSet/$expand?url=.../VMP&filter=...
          GET  /ValueSet/$expand?url=.../AMP&filter=...
          Combined + deduplicated results
      ← NhsDmd::Search::Result
  ← JSON { results: [...] }
Stimulus controller renders result cards
```

The OAuth2 token is fetched once per `NhsDmd::Client` instance
and memoised for the lifetime of that request.

**Rate limit:** 5,000 requests per 5-minute window.
Contact `information.standards@nhs.net` for higher limits.

## Testing locally

Without credentials the feature is gated off and the full test
suite passes — WebMock blocks all real HTTP in the test
environment.

To test the live API locally:

1. Obtain credentials (see [Getting credentials](#getting-credentials)).
2. Export `NHS_DMD_CLIENT_ID` and `NHS_DMD_CLIENT_SECRET`.
3. Run `task dev:up` and sign in as a doctor or administrator.
4. Visit `/medicine-finder` and search for a medicine name,
   for example `Aspirin`.

## Drug interactions

The dm+d API provides **no interaction data** — it is a
medicine catalogue only. A separate data source would be
needed. Options for UK clinical use:

| Source                                     | Notes                         |
|--------------------------------------------|-------------------------------|
| [BNF/NICE](https://bnf.nice.org.uk)        | UK gold standard; NHS licence |
| [OpenFDA](https://open.fda.gov/apis/drug/) | Free; US-focused              |
| [DrugBank](https://go.drugbank.com)        | Comprehensive; commercial     |

The SNOMED CT codes returned by dm+d serve as the bridge —
look up a medicine's code in whichever interactions database
is chosen. Drug interaction lookup is tracked as **MLKP-015**.
