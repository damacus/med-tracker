# Import an NHS dm+d Release

MedTracker imports NHSBSA dm+d release files into its local barcode and product
reference tables. Use the admin upload for routine imports. Use a one-off
Kubernetes Job only when an operator cannot use the admin page.

## Required release files

Download the NHSBSA dm+d release ZIP without changing its contents. The import
expects AMPP data matching `f_ampp2_3*.xml` and either GTIN XML matching
`f_gtin2_0*.xml` or the release's GTIN ZIP.

MedTracker extracts ZIP files with RubyZip. The Rails process does not call the
system `unzip` command.

## Import through the admin page

1. Sign in with platform administration access.
2. Open **Admin**, then **Import NHS dm+d**.
3. Choose the NHSBSA release ZIP exactly as downloaded.
4. Select **Upload and import**.
5. Keep the import page open or return to it to check the latest run.

The upload is stored through the configured Active Storage service. A
background job extracts and imports it. The page shows queued, extraction,
counting, import, completion, or failure status together with record counts and
the import log.

Only one import can be active at a time. Wait for the current run to finish
before uploading another release.

MedTracker deletes the stored archive after a completed or failed run. The
import record and its counts remain available for review.

## Run a headless Kubernetes import

Use a one-off Job when the release must come from object storage or a mounted
volume. The Job must use the same application image and runtime Secret as the
deployed release.

The headless runner accepts an extracted release directory:

```shell
bundle exec rails runner db/seeds/import_nhs_dmd_release.rb /work/nhs-dmd/release
```

Prepare that directory in an init container or read-only volume. Do not run the
import from a web Deployment startup command, every application pod, or a
recurring CronJob.

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: med-tracker-import-dmd-<release>
  namespace: <namespace>
spec:
  backoffLimit: 0
  ttlSecondsAfterFinished: 600
  template:
    spec:
      restartPolicy: Never
      initContainers:
        - name: prepare-release
          image: <artifact-fetcher-image>
          command: ["<fetch-and-extract-command>"]
          volumeMounts:
            - name: release
              mountPath: /work/nhs-dmd
      containers:
        - name: import
          image: ghcr.io/damacus/med-tracker:<image-tag>
          command:
            - bundle
            - exec
            - rails
            - runner
            - db/seeds/import_nhs_dmd_release.rb
            - /work/nhs-dmd/release
          envFrom:
            - secretRef:
                name: med-tracker-runtime
          volumeMounts:
            - name: release
              mountPath: /work/nhs-dmd
              readOnly: true
      volumes:
        - name: release
          emptyDir: {}
```

Replace the init-container image and command with the approved artifact source
for the deployment. Keep object-storage credentials in a Secret, not in the
Job manifest.

## Verify the import

For an admin upload, check that the latest run reaches **Completed** and review
the new, updated, unchanged, and skipped counts.

For a Kubernetes Job, inspect its status and log:

```shell
kubectl get jobs -n <namespace>
kubectl logs job/med-tracker-import-dmd-<release> -n <namespace>
```

The headless log reports imported and skipped totals.

After either flow:

1. Search for a known product from the release.
2. Scan or enter a known GTIN.
3. Confirm that the result resolves to the expected dm+d product.
4. Record the release identifier and import outcome in the operator evidence.

Remove a one-off Job and temporary artifact credentials after a successful
headless import.
