# Kubernetes NHS dm+d Release Import Runbook

Use this guide to import NHS dm+d release XML files into a Kubernetes-hosted
MedTracker production environment.

## TL;DR

1. Upload one dm+d release directory to object storage under a versioned path.
2. Create or update a Secret / ExternalSecret with normal Rails production env
   plus object-storage access credentials.
3. Apply a one-off Kubernetes Job that:
   - downloads the release into an `emptyDir` from an init container
   - expands the GTIN ZIP in that init container if the release is stored as a ZIP
   - runs `bundle exec rails runner db/seeds/import_nhs_dmd_release.rb /work/nhs-dmd/current`
4. Check the Job logs for `Imported:` / `Skipped:` counts.
5. Spot-check a known GTIN in the live app.
6. Remove the one-off Job manifest from GitOps after success.

## What the importer needs

The release importer expects a directory containing:

- `f_ampp2_3*.xml`
- either `f_gtin2_0*.xml` or the GTIN zip file

The Rails entrypoint is:

```bash
bundle exec rails runner db/seeds/import_nhs_dmd_release.rb /work/nhs-dmd/current
```

The importer resolves AMPP product names from the AMPP XML and GTIN mappings
from the GTIN XML. It can also extract the GTIN ZIP, but only when the runtime
environment provides `unzip`.

## Recommended production pattern

Use a **one-off Kubernetes Job per release**.

Do not run the import:

- from the web Deployment startup path
- from an init container on every app pod
- from a CronJob

Why:

- imports are operational and can take time
- pod restarts must not re-trigger imports
- the importer does not currently record a release version marker, so a CronJob
  would just reprocess the same release repeatedly

## Pattern A: object storage + init container + emptyDir

Store each release under a versioned object-storage path, for example:

```text
s3://med-tracker-nhs-dmd/releases/2026-04/current/
```

Prefer storing the release as extracted XML files in object storage.

If you only store the GTIN ZIP, make ZIP extraction part of the init-container
step. The normal app image does not install `unzip`, so do not rely on the
Rails container to unpack production release artifacts.

### Runtime Secret

Use your normal application Secret if it already contains the required Rails env
values. Add object-storage credentials if needed.

Example:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: med-tracker-dmd-import-env
  namespace: <namespace>
type: Opaque
stringData:
  RAILS_ENV: production
  DATABASE_URL: postgresql://...
  SECRET_KEY_BASE: <secret-key-base>
  SOLID_QUEUE_DATABASE_URL: postgresql://...
  SOLID_CACHE_DATABASE_URL: postgresql://...
  SOLID_CABLE_DATABASE_URL: postgresql://...
  AWS_ACCESS_KEY_ID: <access-key>
  AWS_SECRET_ACCESS_KEY: <secret-key>
  AWS_DEFAULT_REGION: eu-west-2
  DMD_RELEASE_URI: s3://med-tracker-nhs-dmd/releases/2026-04/current/
```

### One-off import Job

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: med-tracker-import-dmd-release-2026-04
  namespace: <namespace>
spec:
  backoffLimit: 0
  ttlSecondsAfterFinished: 600
  template:
    spec:
      restartPolicy: Never
      initContainers:
        - name: fetch-dmd-release
          image: amazon/aws-cli:2.22.35
          command:
            - sh
            - -lc
            - |
              mkdir -p /work/nhs-dmd/current
              aws s3 sync "${DMD_RELEASE_URI}" /work/nhs-dmd/current --no-progress
          envFrom:
            - secretRef:
                name: med-tracker-dmd-import-env
          volumeMounts:
            - name: dmd-release
              mountPath: /work/nhs-dmd
      containers:
        - name: import-dmd-release
          image: ghcr.io/damacus/med-tracker:<image-tag>
          command:
            - bundle
            - exec
            - rails
            - runner
            - db/seeds/import_nhs_dmd_release.rb
            - /work/nhs-dmd/current
          envFrom:
            - secretRef:
                name: med-tracker-dmd-import-env
          volumeMounts:
            - name: dmd-release
              mountPath: /work/nhs-dmd
      volumes:
        - name: dmd-release
          emptyDir: {}
```

Notes:

- use the same app image tag as the production release
- if the release path contains a GTIN ZIP instead of extracted XML, use an init
  container image that includes `unzip` and expand it into `/work/nhs-dmd/current`
- if your object store is S3-compatible, add the appropriate endpoint/env flags
  to the init container
- keep the Job manifest in GitOps only until the import succeeds

## Pattern B: read-only PVC mount

If the cluster already exposes release artifacts through a PersistentVolumeClaim,
reuse the same Rails command and skip the download init container.

Requirements:

- mount the PVC read-only into the Job
- expose the release directory at `/work/nhs-dmd/current`

Example container command stays the same:

```yaml
command:
  - bundle
  - exec
  - rails
  - runner
  - db/seeds/import_nhs_dmd_release.rb
  - /work/nhs-dmd/current
```

## Verification

Check Job status:

```bash
kubectl get jobs -n <namespace>
kubectl logs job/med-tracker-import-dmd-release-2026-04 -n <namespace>
```

Expected log output includes:

```text
Imported: <count>, Skipped: <count>
```

Then verify application behavior:

1. Confirm `nhs_dmd_barcodes` row count increased as expected.
2. Search the live finder with a known GTIN from the imported release.
3. Confirm the finder resolves the barcode to a dm+d term and result set.

## Operational workflow per release

1. Upload the new release to object storage or stage it on the PVC.
2. Create or update the import Job manifest with the new release path and a
   unique Job name.
3. Reconcile GitOps or apply the Job directly.
4. Wait for success and inspect logs.
5. Perform the spot checks above.
6. Remove the one-off Job from Git after success.

## Future improvement

If imports need to become scheduled, first add release-version tracking so the
app can detect whether a release has already been imported. Until then, keep
imports as explicit one-off Jobs.
