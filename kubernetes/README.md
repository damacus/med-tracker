# MedTracker Kubernetes Deployment

This directory contains Kubernetes manifests for deploying MedTracker using the
[bjw-s app-template](https://github.com/bjw-s/helm-charts/tree/main/charts/library/common)
Helm chart with FluxCD.

## Structure

```text
kubernetes/
└── apps/
    ├── med-tracker/           # Application deployment
    │   ├── app/
    │   │   ├── HelmRelease.yaml      # App-template Helm release
    │   │   ├── externalsecret.yaml   # 1Password secret sync
    │   │   └── kustomization.yaml
    │   ├── namespace.yaml
    │   └── ks.yaml                   # FluxCD Kustomization
    └── med-tracker-db/        # CloudNativePG database
        ├── app/
        │   ├── cluster.yaml          # PostgreSQL cluster
        │   ├── externalsecret.yaml   # DB credentials
        │   ├── scheduledbackup.yaml  # Daily backups
        │   └── kustomization.yaml
        └── ks.yaml                   # FluxCD Kustomization
```

## Configuration

### Application

- **Route**: `meds.ironstone.casa` (internal gateway)
- **Resources**: 1 CPU / 1GB RAM (limits)
- **Port**: 3000
- **Health checks**: `/up` endpoint

### Database

- **Type**: CloudNativePG PostgreSQL 18
- **Instances**: 2 (HA)
- **Storage**: 10Gi
- **Backups**: Daily to Cloudflare R2

## Required Secrets

Create these in 1Password:

### `med-tracker` (application secrets)

- `SECRET_KEY_BASE` - Rails secret key
- `RAILS_MASTER_KEY` - Rails credentials key

### `med-tracker-db` (database credentials)

- `username` - Database username
- `password` - Database password

The CNPG operator will automatically create a secret `med-tracker-app` with the
connection URI after the cluster is ready.

## Deployment Order

1. `med-tracker-db` - Database cluster (depends on cloudnative-pg, openebs)
2. `med-tracker` - Application (depends on med-tracker-db)

## Optional Persistence

Uncomment the `storage` persistence section in `HelmRelease.yaml` if you need
persistent storage for Active Storage uploads:

```yaml
persistence:
  storage:
    existingClaim: med-tracker-storage
    globalMounts:
      - path: /app/storage
```

Create a PVC beforehand:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: med-tracker-storage
  namespace: med-tracker
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: openebs-hostpath
```

## Usage with home-ops

Copy the `kubernetes/apps/med-tracker` and `kubernetes/apps/med-tracker-db`
directories to your home-ops repository under the appropriate namespace path.
