# NHS dm+d and Medication Lookup

MedTracker uses the NHS Dictionary of Medicines and Devices (dm+d) as its main
UK medication catalogue. It combines dm+d with local barcode records, curated
fallbacks, and Open Food Facts supplement data.

## What dm+d provides

dm+d assigns SNOMED CT identifiers to medicines and packs used in the UK. The
live terminology search can return:

- virtual medicinal products (VMP);
- actual medicinal products (AMP); and
- actual medicinal product packs (AMPP).

Imported release data adds GTIN barcode mappings and trade-family information.
Every search result identifies its source and product. Depending on that source,
it can also include package details, trade-family data, match reasons, product
guidance links, directions, or warnings.

dm+d is a product catalogue. It is not a drug-interaction checker and does not
replace prescribing guidance.

## Lookup order

`NhsDmd::Search` uses several sources:

1. A barcode query first checks the local barcode catalogue populated by dm+d
   release imports and curated records.
2. A barcode without a local match can use Open Food Facts.
3. A text query uses the live NHS terminology service when credentials are
   available.
4. Supplement-like text queries can also use Open Food Facts.

MedTracker merges and deduplicates suitable results. A supplement lookup can
make a remote Open Food Facts request even when NHS credentials are absent.
Do not state that disabling NHS credentials prevents every external lookup.

External lookups write source, outcome, and result-count metadata to the audit
trail. Query text is handled by the external-lookup audit policy.

## NHS terminology credentials

Live dm+d text search needs a system-to-system account from the NHS England
Terminology Server. This is separate from the NHS API Platform.

1. Read the [system-to-system account
   agreement](https://digital.nhs.uk/services/terminology-server/system-to-system-account-agreement).
2. Complete the [account request
   form](https://digital.nhs.uk/services/terminology-server/request-a-system-to-system-account/request-form).
3. Request read-only consumer access for the application's clinical use.
4. Store the issued client ID and secret outside the repository.

Configure both values:

| Variable | Purpose |
| --- | --- |
| `NHS_DMD_CLIENT_ID` | OAuth client ID from NHS England |
| `NHS_DMD_CLIENT_SECRET` | OAuth client secret |

Set Fish shell variables before starting local development:

```fish
set -x NHS_DMD_CLIENT_ID "your-client-id"
set -x NHS_DMD_CLIENT_SECRET "your-client-secret"
task dev:up
```

Use a Secret or ExternalSecret in production. Never put either value in a
ConfigMap, image, or committed environment file.

When one of these values is absent, live NHS terminology search is unavailable.
Local barcode data, curated data, and eligible supplement lookups can still
return results.

## Import NHS release data

Platform administrators can upload the NHSBSA release ZIP through **Admin**,
then **Import NHS dm+d**. MedTracker stores the archive, queues the import,
shows progress and record counts, and removes the archive when the run ends.

Kubernetes operators can use a one-off Job with an extracted release directory
when the admin upload is unsuitable. See [Import an NHS dm+d
release](kubernetes-nhs-dmd-import.md).

Local development provides Task wrappers for a staged release:

```fish
task dev:extract-dmd-release
task dev:import-dmd-release RELEASE_DIR=storage/nhs_dmd/releases/current
```

## Verify the integration

1. Search for a known generic medicine and confirm a VMP result.
2. Search for a known branded product and confirm an AMP or AMPP result.
3. Enter a GTIN from the imported release and confirm the expected pack.
4. Try a supplement query and identify its source before saving it.
5. Confirm that failed external lookups do not expose credentials in logs.

Automated tests stub remote services. A green test suite does not prove that
deployment credentials or NHS access are valid.

## Product guidance and interactions

Some source records include patient-information or product-characteristic
links. Their presence depends on the returned product data.

MedTracker does not provide a drug-interaction database. Any future interaction
feature needs a separately licensed and clinically governed source. SNOMED CT
or dm+d identifiers can link the selected medication to that source.
