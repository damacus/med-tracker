# Kubernetes User Seeding Runbook

Use this guide to seed MedTracker users in Kubernetes quickly and safely.

## TL;DR (for on-call/sysadmin use)

1. **Bootstrap first admin** (first deploy only): create Secret with `ADMIN_*` vars and run bootstrap Job (`rails med_tracker:bootstrap_admin`).
2. Create `users.yml` with invite targets (ConfigMap).
3. Create runtime secrets (`APP_URL`, SMTP, etc.) via Secret or ExternalSecret.
4. Run a one-off Job using the production image and `rails db:seed`.
5. Check Job logs and confirm invited users/admin access.
6. Remove one-off Job manifests from GitOps after success.

## What gets seeded

MedTracker production seeding (`db/seeds.rb`) does two things:

1. Seeds medicine reference data.
2. Invites initial users from `db/seeds/users.yml`.

Relevant behavior:

- `db/seeds.rb` runs `db/seeds/seed_users.rb` in production.
- `db/seeds/seed_users.rb` is idempotent:
  - Skips if an account already exists for email.
  - Skips if a pending invitation already exists for email.

This means re-running the Job is safe.

---

## Prerequisites

- Kubernetes cluster + namespace for MedTracker.
- MedTracker app image available (e.g. `ghcr.io/damacus/med-tracker:v0.3.1`).
- Database connectivity and `DATABASE_URL` available to the Job.
- Required mailer/env vars for sending invites.
- If using External Secrets: External Secrets Operator installed and reconciled.

---

## Choose the right flow

| Goal                           | Command path                                       | Inputs                                                     |
|--------------------------------|----------------------------------------------------|------------------------------------------------------------|
| Bootstrap first admin account  | `rails med_tracker:bootstrap_admin`                | `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `ADMIN_NAME`, `ADMIN_DOB` |
| Invite initial users from YAML | `rails db:seed` (or `load db/seeds/seed_users.rb`) | `db/seeds/users.yml` + mailer/app env                      |

For first-time production setup, you often do both:

1. Bootstrap admin.
2. Seed invitations for the broader care team.

---

## First system administrator bootstrap (do this first)

If no administrator exists yet, run this flow before invite seeding.

Required variables:

- `ADMIN_EMAIL`
- `ADMIN_PASSWORD`
- `ADMIN_NAME`
- `ADMIN_DOB` (`YYYY-MM-DD`)

### Bootstrap Secret (or ExternalSecret target)

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: med-tracker-bootstrap-admin
  namespace: <namespace>
type: Opaque
stringData:
  RAILS_ENV: production
  DATABASE_URL: postgresql://...
  ADMIN_EMAIL: admin@yourdomain.example
  ADMIN_PASSWORD: <strong-random-password>
  ADMIN_NAME: System Administrator
  ADMIN_DOB: "1980-01-01"
```

### One-off bootstrap Job

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: med-tracker-bootstrap-admin
  namespace: <namespace>
spec:
  backoffLimit: 0
  ttlSecondsAfterFinished: 600
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: bootstrap-admin
          image: ghcr.io/damacus/med-tracker:<image-tag>
          command: ["bundle", "exec", "rails", "med_tracker:bootstrap_admin"]
          envFrom:
            - secretRef:
                name: med-tracker-bootstrap-admin
```

Verification:

```bash
kubectl logs job/med-tracker-bootstrap-admin -n <namespace>
```

Expected output includes `Admin bootstrap successful: created <email>`.

If using External Secrets, create an `ExternalSecret` that writes to
`med-tracker-bootstrap-admin` with the same keys above, then run the same Job.

---

## Pattern A: ConfigMap + Secret

### 1) Create `users.yml`

Example `users.yml`:

```yaml
---
- email: admin@yourdomain.example
  role: administrator
- email: nurse.lead@yourdomain.example
  role: nurse
- email: carer.team@yourdomain.example
  role: carer
```

Supported roles: `administrator`, `doctor`, `nurse`, `carer`, `parent`.

### 2) ConfigMap with `users.yml`

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: med-tracker-seed-users
  namespace: <namespace>
data:
  users.yml: |
    ---
    - email: admin@yourdomain.example
      role: administrator
    - email: nurse.lead@yourdomain.example
      role: nurse
```

### 3) Secret for runtime env

Use your existing app Secret if it already includes required env vars. Otherwise create a dedicated one:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: med-tracker-seed-env
  namespace: <namespace>
type: Opaque
stringData:
  RAILS_ENV: production
  APP_URL: https://app.yourdomain.example
  DATABASE_URL: postgresql://...
  SMTP_ADDRESS: smtp.example.com
  SMTP_PORT: "587"
  SMTP_USERNAME: <smtp-user>
  SMTP_PASSWORD: <smtp-password>
  SMTP_DOMAIN: yourdomain.example
```

### 4) One-off seeding Job

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: med-tracker-seed-users
  namespace: <namespace>
spec:
  backoffLimit: 0
  ttlSecondsAfterFinished: 600
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: seed-users
          image: ghcr.io/damacus/med-tracker:<image-tag>
          command: ["bundle", "exec", "rails", "db:seed"]
          envFrom:
            - secretRef:
                name: med-tracker-seed-env
          volumeMounts:
            - name: seed-users-file
              mountPath: /app/db/seeds/users.yml
              subPath: users.yml
              readOnly: true
      volumes:
        - name: seed-users-file
          configMap:
            name: med-tracker-seed-users
```

---

## Pattern B: ExternalSecret + ConfigMap

Use this when secrets are managed outside the cluster.

### 1) ExternalSecret (provider-agnostic skeleton)

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: med-tracker-seed-env
  namespace: <namespace>
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: <cluster-secret-store-name>
    kind: ClusterSecretStore
  target:
    name: med-tracker-seed-env
    creationPolicy: Owner
  data:
    - secretKey: APP_URL
      remoteRef:
        key: med-tracker/prod/app-url
    - secretKey: DATABASE_URL
      remoteRef:
        key: med-tracker/prod/database-url
    - secretKey: SMTP_ADDRESS
      remoteRef:
        key: med-tracker/prod/smtp-address
    - secretKey: SMTP_PORT
      remoteRef:
        key: med-tracker/prod/smtp-port
    - secretKey: SMTP_USERNAME
      remoteRef:
        key: med-tracker/prod/smtp-username
    - secretKey: SMTP_PASSWORD
      remoteRef:
        key: med-tracker/prod/smtp-password
    - secretKey: SMTP_DOMAIN
      remoteRef:
        key: med-tracker/prod/smtp-domain
```

### 2) Reuse the same seeding Job

Use the same Job manifest as Pattern A; it references `med-tracker-seed-env` via `envFrom.secretRef`.

### 3) Ordering note

In GitOps, ensure `ExternalSecret` reconciles before the Job runs. If needed:

- Apply ExternalSecret/Kustomization first.
- Confirm generated Secret exists.
- Then apply the one-off Job.

---

## Verification

```bash
kubectl get jobs -n <namespace>
kubectl describe job med-tracker-seed-users -n <namespace>
kubectl logs job/med-tracker-seed-users -n <namespace>
```

Expected log patterns include:

- `Invited <email> as <role>.`
- `Skipping <email> — account already exists.`
- `Skipping <email> — pending invitation already exists.`
- `User seeding complete: X invited, Y skipped.`

Application-level checks:

1. Admin can sign in and access `/admin`.
2. Invited users received email (or queued delivery observed).
3. No unexpected duplicate invitations.

---

## Cleanup after success

1. Remove/disable the one-off Job manifest from GitOps.
2. Keep `users.yml` source controlled (or archived) for auditability.
3. Rotate secrets per normal policy (especially SMTP and DB credentials if temporary).

---

## Troubleshooting

| Symptom                                   | Likely cause                                       | Action                                                        |
|-------------------------------------------|----------------------------------------------------|---------------------------------------------------------------|
| Job failed immediately                    | Missing env vars (`DATABASE_URL`, `APP_URL`, SMTP) | Check Secret/ExternalSecret values and re-run Job             |
| Job cannot find `/app/db/seeds/users.yml` | ConfigMap mount path/subPath mismatch              | Verify `mountPath` and `subPath: users.yml`                   |
| No invites sent                           | `users.yml` malformed or empty                     | Validate YAML structure and role values                       |
| Users skipped unexpectedly                | Existing account or pending invitation             | Expected idempotent behavior; review logs                     |
| ExternalSecret present but Secret missing | ESO reconciliation/store auth issue                | Check `kubectl describe externalsecret ...` and operator logs |
| Mail delivery errors                      | SMTP credentials/network policy issue              | Validate SMTP env + egress/network policies                   |

---

## Security notes

- Do not store credentials in ConfigMaps.
- Keep secrets in Secret/ExternalSecret backends.
- Restrict Job ServiceAccount/RBAC to minimum required permissions.
- Prefer short-lived one-off Jobs; remove manifests once complete.

---

## Related docs

- Deployment: `docs/deployment.md`
- User model/roles: `docs/user-management.md`
- Existing seed file template: `db/seeds/users.yml`
