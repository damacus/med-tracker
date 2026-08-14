# Kubernetes User Seeding

Use one-off Kubernetes Jobs to create the first household owner and invite
initial household members. Use the same application image and runtime settings
as the deployed MedTracker release.

## Before you begin

The Job needs production database access and the same encryption, URL, and mail
settings as the application. Keep credentials in a Secret or ExternalSecret.
Do not put passwords or database URLs in a ConfigMap.

## Create the first household owner

Run this step only when no active household owner exists. The bootstrap task
creates the first account, person, household, and owner membership.

Required values:

- `ADMIN_EMAIL`
- `ADMIN_PASSWORD`
- `ADMIN_NAME`
- `ADMIN_DOB` in `YYYY-MM-DD` format

Include `RAILS_ENV=production`, `DATABASE_URL`, and the application's normal
production settings in the Job environment.

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
                name: med-tracker-runtime
            - secretRef:
                name: med-tracker-bootstrap-admin
```

The command refuses to run when any active owner membership already exists. It
also refuses duplicate email addresses or incomplete input.

Check the Job log, confirm that the owner can sign in, then delete the bootstrap
Secret and Job from the GitOps source.

## Invite initial members

Production `rails db:seed` loads default locations and medication reference
data before reading `db/seeds/users.yml`. Run the complete seed command only
when those effects are intended.

Create a `users.yml` file with the accounts to invite:

```yaml
---
- email: admin@yourdomain.example
  membership_role: administrator
- email: carer@yourdomain.example
  membership_role: member
```

The supported invitation roles are `administrator` and `member`. Owner access
cannot be granted through an invitation.

Mount the file at `/app/db/seeds/users.yml` with a ConfigMap:

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
      membership_role: administrator
    - email: carer@yourdomain.example
      membership_role: member
```

Run the seed Job with the normal runtime Secret:

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
          env:
            - name: HOUSEHOLD_SLUG
              value: <household-slug>
          envFrom:
            - secretRef:
                name: med-tracker-runtime
          volumeMounts:
            - name: seed-users
              mountPath: /app/db/seeds/users.yml
              subPath: users.yml
              readOnly: true
      volumes:
        - name: seed-users
          configMap:
            name: med-tracker-seed-users
```

Set either `HOUSEHOLD_SLUG` or `HOUSEHOLD_ID`. Without a selector, the seed uses
the oldest household, which is unsafe when more than one household exists.

The invitation step skips an email when its account already exists or the
selected household already has a pending invitation for it.

## Verify and clean up

```shell
kubectl get jobs -n <namespace>
kubectl describe job med-tracker-seed-users -n <namespace>
kubectl logs job/med-tracker-seed-users -n <namespace>
```

Confirm that each invitation belongs to the selected household and has the
expected membership role. Check that invitation delivery was queued or sent.

Remove the one-off Job and its ConfigMap from the GitOps source after the run.
Keep the invitation list only when your audit policy requires it, and do not
store personal email addresses in a public repository.
