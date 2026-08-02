#!/usr/bin/env fish

if test (count $argv) -ne 2
    echo 'Usage: production_storage_smoke.fish IMAGE_REF disk|s3' >&2
    exit 2
end

for command_name in docker
    if not command -q $command_name
        echo "Required command is unavailable: $command_name" >&2
        exit 2
    end
end

set -g STORAGE_SMOKE_IMAGE $argv[1]
set -g STORAGE_SMOKE_MODE $argv[2]
set -g STORAGE_SMOKE_ROOT (realpath (dirname (status filename))/..)
set -g STORAGE_SMOKE_SUFFIX (random)
set -g STORAGE_SMOKE_PREFIX "storage-smoke-$STORAGE_SMOKE_SUFFIX"
set -g STORAGE_SMOKE_NETWORK "$STORAGE_SMOKE_PREFIX-network"
set -g STORAGE_SMOKE_DB "$STORAGE_SMOKE_PREFIX-db"
set -g STORAGE_SMOKE_DISK_VOLUME "$STORAGE_SMOKE_PREFIX-disk"
set -g STORAGE_SMOKE_S3_VOLUME "$STORAGE_SMOKE_PREFIX-s3"
set -g STORAGE_SMOKE_S3_CONTAINER "$STORAGE_SMOKE_PREFIX-rustfs"

function cleanup_storage_smoke --on-event fish_exit
    docker rm -f $STORAGE_SMOKE_S3_CONTAINER $STORAGE_SMOKE_DB >/dev/null 2>&1
    docker network rm $STORAGE_SMOKE_NETWORK >/dev/null 2>&1
    docker volume rm $STORAGE_SMOKE_DISK_VOLUME $STORAGE_SMOKE_S3_VOLUME >/dev/null 2>&1
end

if not contains $STORAGE_SMOKE_MODE disk s3
    echo "Unsupported storage smoke mode: $STORAGE_SMOKE_MODE" >&2
    exit 2
end

docker network create $STORAGE_SMOKE_NETWORK >/dev/null
or exit 1

docker run --detach \
    --name $STORAGE_SMOKE_DB \
    --network $STORAGE_SMOKE_NETWORK \
    --env POSTGRES_USER=medtracker \
    --env POSTGRES_PASSWORD=medtracker_password \
    --env POSTGRES_DB=medtracker \
    --env POSTGRES_MULTIPLE_DATABASES=medtracker_production_queue,medtracker_production_cache,medtracker_production_cable \
    --volume "$STORAGE_SMOKE_ROOT/compose/init-roles.sql:/docker-entrypoint-initdb.d/001-init-roles.sql:ro" \
    --volume "$STORAGE_SMOKE_ROOT/compose/init-multiple-dbs.sh:/docker-entrypoint-initdb.d/002-init-multiple-dbs.sh:ro" \
    postgres:18-alpine >/dev/null
or exit 1

for attempt in (seq 1 60)
    if docker exec $STORAGE_SMOKE_DB pg_isready -U medtracker -d medtracker >/dev/null 2>&1
        break
    end
    if test $attempt -eq 60
        docker logs $STORAGE_SMOKE_DB
        echo 'PostgreSQL did not become ready' >&2
        exit 1
    end
    sleep 1
end

docker volume create $STORAGE_SMOKE_DISK_VOLUME >/dev/null
or exit 1

set -g STORAGE_SMOKE_COMMON_ENV \
    --env RAILS_ENV=production \
    --env APP_URL=http://localhost \
    --env APP_VERSION=$STORAGE_SMOKE_IMAGE \
    --env SECRET_KEY_BASE=production-storage-smoke-secret \
    --env DATABASE_URL=postgresql://medtracker:medtracker_password@$STORAGE_SMOKE_DB:5432/medtracker \
    --env DATABASE_ROLE= \
    --env SOLID_QUEUE_DATABASE_URL=postgresql://medtracker:medtracker_password@$STORAGE_SMOKE_DB:5432/medtracker_production_queue \
    --env SOLID_CACHE_DATABASE_URL=postgresql://medtracker:medtracker_password@$STORAGE_SMOKE_DB:5432/medtracker_production_cache \
    --env SOLID_CABLE_DATABASE_URL=postgresql://medtracker:medtracker_password@$STORAGE_SMOKE_DB:5432/medtracker_production_cable

docker run --rm \
    --network $STORAGE_SMOKE_NETWORK \
    $STORAGE_SMOKE_COMMON_ENV \
    --env ACTIVE_STORAGE_SERVICE=persistent \
    --env ACTIVE_STORAGE_ROOT=/app/storage \
    --volume "$STORAGE_SMOKE_DISK_VOLUME:/app/storage" \
    $STORAGE_SMOKE_IMAGE \
    bin/rails db:prepare
or exit 1

set -g STORAGE_SMOKE_RUNNER 'require "digest"; require "stringio"; key = "storage-smoke/#{SecureRandom.uuid}"; payload = "portable-storage-smoke"; service = ActiveStorage::Blob.services.fetch(Rails.configuration.active_storage.service); checksum = Digest::MD5.base64digest(payload); begin; service.upload(key, StringIO.new(payload), checksum: checksum); raise "download mismatch" unless service.download(key) == payload; ensure; service.delete(key); end; raise "delete failed" if service.exist?(key); puts "storage smoke passed"'

if test $STORAGE_SMOKE_MODE = disk
    set -l disk_runner 'mounts = File.readlines("/proc/self/mountinfo"); raise "Disk smoke is missing /app/storage mount" unless mounts.any? { |line| line.split.fetch(4) == "/app/storage" }; '"$STORAGE_SMOKE_RUNNER"
    docker run --rm \
        --network $STORAGE_SMOKE_NETWORK \
        $STORAGE_SMOKE_COMMON_ENV \
        --env ACTIVE_STORAGE_SERVICE=persistent \
        --env ACTIVE_STORAGE_ROOT=/app/storage \
        --volume "$STORAGE_SMOKE_DISK_VOLUME:/app/storage" \
        $STORAGE_SMOKE_IMAGE \
        bin/rails runner $disk_runner
    or exit 1

    echo 'Final production-image Disk storage smoke passed'
    exit 0
end

docker volume create $STORAGE_SMOKE_S3_VOLUME >/dev/null
or exit 1

docker run --detach \
    --name $STORAGE_SMOKE_S3_CONTAINER \
    --network $STORAGE_SMOKE_NETWORK \
    --network-alias rustfs \
    --env RUSTFS_ACCESS_KEY=storage-smoke-access \
    --env RUSTFS_SECRET_KEY=storage-smoke-secret \
    --env RUSTFS_VOLUMES=/data \
    --volume "$STORAGE_SMOKE_S3_VOLUME:/data" \
    rustfs/rustfs:1.0.0-alpha.90 /data >/dev/null
or exit 1

set -g STORAGE_SMOKE_S3_ENV \
    --env ACTIVE_STORAGE_SERVICE=s3 \
    --env ACTIVE_STORAGE_S3_ENDPOINT=http://rustfs:9000 \
    --env ACTIVE_STORAGE_S3_BUCKET=medtracker-storage-smoke \
    --env ACTIVE_STORAGE_S3_REGION=us-east-1 \
    --env ACTIVE_STORAGE_S3_ACCESS_KEY_ID=storage-smoke-access \
    --env ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY=storage-smoke-secret \
    --env ACTIVE_STORAGE_S3_FORCE_PATH_STYLE=true

set -l bucket_runner 'require "aws-sdk-s3"; client = Aws::S3::Client.new(endpoint: ENV.fetch("ACTIVE_STORAGE_S3_ENDPOINT"), region: ENV.fetch("ACTIVE_STORAGE_S3_REGION"), access_key_id: ENV.fetch("ACTIVE_STORAGE_S3_ACCESS_KEY_ID"), secret_access_key: ENV.fetch("ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY"), force_path_style: true); client.create_bucket(bucket: ENV.fetch("ACTIVE_STORAGE_S3_BUCKET"))'
set -l rustfs_ready false
for attempt in (seq 1 60)
    docker run --rm \
        --network $STORAGE_SMOKE_NETWORK \
        $STORAGE_SMOKE_COMMON_ENV \
        $STORAGE_SMOKE_S3_ENV \
        $STORAGE_SMOKE_IMAGE \
        bin/rails runner $bucket_runner >/dev/null 2>&1
    if test $status -eq 0
        set rustfs_ready true
        break
    end
    sleep 1
end

if test $rustfs_ready != true
    docker logs $STORAGE_SMOKE_S3_CONTAINER
    echo 'RustFS did not become ready for the S3 smoke' >&2
    exit 1
end

set -l s3_runner 'mounts = File.readlines("/proc/self/mountinfo"); raise "S3 smoke unexpectedly mounted /app/storage" if mounts.any? { |line| line.split.fetch(4) == "/app/storage" }; '"$STORAGE_SMOKE_RUNNER"
docker run --rm \
    --network $STORAGE_SMOKE_NETWORK \
    $STORAGE_SMOKE_COMMON_ENV \
    $STORAGE_SMOKE_S3_ENV \
    $STORAGE_SMOKE_IMAGE \
    bin/rails runner $s3_runner
or exit 1

echo 'Final production-image S3 storage smoke passed'
