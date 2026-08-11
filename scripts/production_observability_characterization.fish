#!/usr/bin/env fish

if test (count $argv) -ne 1
    echo 'Usage: production_observability_characterization.fish IMAGE_REF' >&2
    exit 2
end

for command_name in docker curl jq
    if not command -q $command_name
        echo "Required command is unavailable: $command_name" >&2
        exit 2
    end
end

set -g OBSERVABILITY_CHARACTERIZATION_IMAGE $argv[1]
set -g OBSERVABILITY_CHARACTERIZATION_ROOT (realpath (dirname (status filename))/..)
set -g OBSERVABILITY_CHARACTERIZATION_TMP (mktemp -d)
set -g OBSERVABILITY_CHARACTERIZATION_SUFFIX (random)
set -g OBSERVABILITY_CHARACTERIZATION_PREFIX "observability-characterization-$OBSERVABILITY_CHARACTERIZATION_SUFFIX"
set -g OBSERVABILITY_CHARACTERIZATION_NETWORK "$OBSERVABILITY_CHARACTERIZATION_PREFIX-network"
set -g OBSERVABILITY_CHARACTERIZATION_DB "$OBSERVABILITY_CHARACTERIZATION_PREFIX-db"
set -g OBSERVABILITY_CHARACTERIZATION_RECEIVER "$OBSERVABILITY_CHARACTERIZATION_PREFIX-receiver"
set -g OBSERVABILITY_CHARACTERIZATION_APP "$OBSERVABILITY_CHARACTERIZATION_PREFIX-app"
set -g OBSERVABILITY_CHARACTERIZATION_WORKER "$OBSERVABILITY_CHARACTERIZATION_PREFIX-worker"
set -g OBSERVABILITY_CHARACTERIZATION_STORAGE "$OBSERVABILITY_CHARACTERIZATION_PREFIX-storage"

mkdir -p $OBSERVABILITY_CHARACTERIZATION_TMP/otlp
or exit 1

function cleanup_observability_characterization --on-event fish_exit
    docker rm -f \
        $OBSERVABILITY_CHARACTERIZATION_APP \
        $OBSERVABILITY_CHARACTERIZATION_WORKER \
        $OBSERVABILITY_CHARACTERIZATION_RECEIVER \
        $OBSERVABILITY_CHARACTERIZATION_DB >/dev/null 2>&1
    docker network rm $OBSERVABILITY_CHARACTERIZATION_NETWORK >/dev/null 2>&1
    docker volume rm $OBSERVABILITY_CHARACTERIZATION_STORAGE >/dev/null 2>&1
    if test -d $OBSERVABILITY_CHARACTERIZATION_TMP
        and string match -qr '^/(tmp|private/tmp|private/var/folders)/' $OBSERVABILITY_CHARACTERIZATION_TMP
        rm -rf -- $OBSERVABILITY_CHARACTERIZATION_TMP
    end
end

string join \n \
    'events {}' \
    'user root;' \
    'http {' \
    '  client_body_in_file_only on;' \
    '  client_body_temp_path /var/cache/nginx/client_temp 1 2;' \
    "  log_format otlp '\$request_method \$uri \$request_body_file \$http_content_encoding';" \
    '  access_log /dev/stdout otlp;' \
    '  server {' \
    '    listen 4318;' \
    '    location / {' \
    '      proxy_pass http://127.0.0.1:4319;' \
    '    }' \
    '  }' \
    '  server {' \
    '    listen 4319;' \
    '    location / {' \
    '      return 200;' \
    '    }' \
    '  }' \
    '}' \
    >$OBSERVABILITY_CHARACTERIZATION_TMP/nginx.conf

docker network create $OBSERVABILITY_CHARACTERIZATION_NETWORK >/dev/null
or exit 1
docker volume create $OBSERVABILITY_CHARACTERIZATION_STORAGE >/dev/null
or exit 1

docker run --detach \
    --name $OBSERVABILITY_CHARACTERIZATION_DB \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    --env POSTGRES_USER=medtracker \
    --env POSTGRES_PASSWORD=medtracker_password \
    --env POSTGRES_DB=medtracker \
    --env POSTGRES_MULTIPLE_DATABASES=medtracker_production_queue,medtracker_production_cache,medtracker_production_cable \
    --volume "$OBSERVABILITY_CHARACTERIZATION_ROOT/compose/init-roles.sql:/docker-entrypoint-initdb.d/001-init-roles.sql:ro" \
    --volume "$OBSERVABILITY_CHARACTERIZATION_ROOT/compose/init-multiple-dbs.sh:/docker-entrypoint-initdb.d/002-init-multiple-dbs.sh:ro" \
    postgres:18-alpine >/dev/null
or exit 1

for attempt in (seq 1 60)
    if docker exec $OBSERVABILITY_CHARACTERIZATION_DB pg_isready -U medtracker -d medtracker >/dev/null 2>&1
        break
    end
    if test $attempt -eq 60
        docker logs $OBSERVABILITY_CHARACTERIZATION_DB
        echo 'PostgreSQL did not become ready' >&2
        exit 1
    end
    sleep 1
end

docker run --detach \
    --name $OBSERVABILITY_CHARACTERIZATION_RECEIVER \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    --network-alias otel-receiver \
    --volume "$OBSERVABILITY_CHARACTERIZATION_TMP/nginx.conf:/etc/nginx/nginx.conf:ro" \
    --volume "$OBSERVABILITY_CHARACTERIZATION_TMP/otlp:/var/cache/nginx/client_temp" \
    nginx:1.29-alpine >/dev/null
or exit 1

set -g OBSERVABILITY_CHARACTERIZATION_ENV \
    --env RAILS_ENV=production \
    --env APP_URL=http://localhost \
    --env ACTIVE_STORAGE_SERVICE=persistent \
    --env ACTIVE_STORAGE_ROOT=/app/storage \
    --env DATABASE_URL=postgresql://medtracker:medtracker_password@$OBSERVABILITY_CHARACTERIZATION_DB:5432/medtracker \
    --env DATABASE_ROLE= \
    --env SECRET_KEY_BASE=observability-characterization-secret \
    --env SOLID_QUEUE_DATABASE_URL=postgresql://medtracker:medtracker_password@$OBSERVABILITY_CHARACTERIZATION_DB:5432/medtracker_production_queue \
    --env SOLID_CACHE_DATABASE_URL=postgresql://medtracker:medtracker_password@$OBSERVABILITY_CHARACTERIZATION_DB:5432/medtracker_production_cache \
    --env SOLID_CABLE_DATABASE_URL=postgresql://medtracker:medtracker_password@$OBSERVABILITY_CHARACTERIZATION_DB:5432/medtracker_production_cable \
    --env OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-receiver:4318 \
    --env OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf \
    --env OTEL_TRACES_EXPORTER=otlp \
    --env OTEL_METRICS_EXPORTER=otlp \
    --env APP_VERSION=$OBSERVABILITY_CHARACTERIZATION_IMAGE

set -g OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    --volume "$OBSERVABILITY_CHARACTERIZATION_STORAGE:/app/storage"

docker run --rm \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    $OBSERVABILITY_CHARACTERIZATION_ENV \
    $OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    bin/rails db:prepare >$OBSERVABILITY_CHARACTERIZATION_TMP/migrate.log 2>&1
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/migrate.log
    exit 1
end

docker run --detach \
    --name $OBSERVABILITY_CHARACTERIZATION_WORKER \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    $OBSERVABILITY_CHARACTERIZATION_ENV \
    $OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    bin/jobs >/dev/null
or exit 1

docker run --detach \
    --name $OBSERVABILITY_CHARACTERIZATION_APP \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    --publish 127.0.0.1::80 \
    $OBSERVABILITY_CHARACTERIZATION_ENV \
    $OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE >/dev/null
or exit 1

set -l app_port (docker port $OBSERVABILITY_CHARACTERIZATION_APP 80/tcp | string replace -r '^.*:' '')
set -l app_url "http://localhost:$app_port"

for attempt in (seq 1 60)
    if curl --silent --fail "$app_url/up" >/dev/null 2>&1
        break
    end
    if test $attempt -eq 60
        docker logs $OBSERVABILITY_CHARACTERIZATION_APP
        echo 'Application did not become ready' >&2
        exit 1
    end
    sleep 1
end

if docker inspect --format '{{range .Mounts}}{{println .Type .Destination}}{{end}}' \
        $OBSERVABILITY_CHARACTERIZATION_APP |
        awk '$1 == "bind" && $2 ~ "^/app" { found = 1 } END { exit found }'
    true
else
    echo 'Application container has a repository bind mount' >&2
    exit 1
end

curl --silent --show-error \
    --header 'X-Request-Id: observability-characterization-request' \
    --header 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
    "$app_url/login" \
    --output /dev/null
or exit 1

set -l runner_code 'Rails.logger.warn("observability-characterization-warning"); Rails.logger.error("observability-characterization-error"); ActiveSupport::Notifications.instrument("take_attempted.med_tracker", source_type: "characterization"); tracer = OpenTelemetry.tracer_provider.tracer("observability-characterization"); tracer.in_span("medication_take.characterization") { |span| span.set_attribute("verification.kind", "synthetic") }; ScheduleDailyRemindersJob.perform_later; OpenTelemetry.tracer_provider.force_flush'

docker run --rm \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    $OBSERVABILITY_CHARACTERIZATION_ENV \
    $OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    bin/rails runner $runner_code >$OBSERVABILITY_CHARACTERIZATION_TMP/runner.log 2>&1
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/runner.log
    exit 1
end

for attempt in (seq 1 30)
    if docker logs $OBSERVABILITY_CHARACTERIZATION_WORKER 2>&1 |
            string match -q '*ScheduleDailyRemindersJob*'
        break
    end
    if test $attempt -eq 30
        docker logs $OBSERVABILITY_CHARACTERIZATION_WORKER
        echo 'Solid Queue did not process the characterization job' >&2
        exit 1
    end
    sleep 1
end

set -l canary_trace_body_paths_before (
    docker logs $OBSERVABILITY_CHARACTERIZATION_RECEIVER 2>&1 |
        awk '$1 == "POST" && $2 == "/canary/v1/traces" && $3 != "-" { sub("^/var/cache/nginx/client_temp", "/otlp", $3); print $3 }'
)

docker run --rm \
    --network $OBSERVABILITY_CHARACTERIZATION_NETWORK \
    $OBSERVABILITY_CHARACTERIZATION_ENV \
    $OBSERVABILITY_CHARACTERIZATION_MOUNTS \
    --env OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-receiver:4318/canary \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    bin/rails observability:canary >$OBSERVABILITY_CHARACTERIZATION_TMP/canary.log 2>&1
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/canary.log
    exit 1
end

set -l canary_trace_body_paths_after (
    docker logs $OBSERVABILITY_CHARACTERIZATION_RECEIVER 2>&1 |
        awk '$1 == "POST" && $2 == "/canary/v1/traces" && $3 != "-" { sub("^/var/cache/nginx/client_temp", "/otlp", $3); print $3 }'
)

set -l canary_trace_body_paths
for path in $canary_trace_body_paths_after
    if not contains -- $path $canary_trace_body_paths_before
        set --append canary_trace_body_paths $path
    end
end

if test (count $canary_trace_body_paths) -eq 0
    echo 'Canary command did not export an enqueue-side observability trace' >&2
    exit 1
end

set -l canary_trace_body_paths_env (string join : $canary_trace_body_paths)

docker exec $OBSERVABILITY_CHARACTERIZATION_RECEIVER chmod -R a+rX /var/cache/nginx/client_temp
or exit 1

docker run --rm \
    --entrypoint ruby \
    --volume "$OBSERVABILITY_CHARACTERIZATION_TMP/otlp:/otlp:ro" \
    --env "OTLP_TRACE_FILES=$canary_trace_body_paths_env" \
    --env OTLP_REQUIRED_SPAN_NAME=observability.canary \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    scripts/verify_otlp_trace_resources.rb
or begin
    echo 'Canary command did not export an enqueue-side observability trace' >&2
    exit 1
end

docker logs $OBSERVABILITY_CHARACTERIZATION_APP \
    >$OBSERVABILITY_CHARACTERIZATION_TMP/app.log 2>&1
docker logs $OBSERVABILITY_CHARACTERIZATION_WORKER \
    >$OBSERVABILITY_CHARACTERIZATION_TMP/worker.log 2>&1
docker logs $OBSERVABILITY_CHARACTERIZATION_RECEIVER \
    >$OBSERVABILITY_CHARACTERIZATION_TMP/receiver.log 2>&1

string match -q '*observability-characterization-warning*' \
    <$OBSERVABILITY_CHARACTERIZATION_TMP/runner.log
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/runner.log
    echo 'Production logger output was not captured' >&2
    exit 1
end

string match -q '*POST /v1/traces*' \
    <$OBSERVABILITY_CHARACTERIZATION_TMP/receiver.log
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/runner.log
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/worker.log
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/receiver.log
    echo 'OpenTelemetry trace exporter traffic was not captured' >&2
    exit 1
end

docker run --rm \
    --entrypoint ruby \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    -e 'abort "Final image does not contain the host.name resource assignment" unless File.read("config/initializers/opentelemetry.rb").include?("host.name")'
or exit 1

set -l trace_body_paths (
    awk '$1 == "POST" && $2 == "/v1/traces" && $3 != "-" { sub("^/var/cache/nginx/client_temp", "/otlp", $3); print $3 }' \
        $OBSERVABILITY_CHARACTERIZATION_TMP/receiver.log
)

if test (count $trace_body_paths) -eq 0
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/receiver.log
    echo 'OpenTelemetry trace exporter body was not captured' >&2
    exit 1
end

set -l trace_body_paths_env (string join : $trace_body_paths)

docker exec $OBSERVABILITY_CHARACTERIZATION_RECEIVER chmod -R a+rX /var/cache/nginx/client_temp
or exit 1

docker run --rm \
    --entrypoint ruby \
    --volume "$OBSERVABILITY_CHARACTERIZATION_TMP/otlp:/otlp:ro" \
    --env "OTLP_TRACE_FILES=$trace_body_paths_env" \
    $OBSERVABILITY_CHARACTERIZATION_IMAGE \
    scripts/verify_otlp_trace_resources.rb
or exit 1

jq --raw-input --slurp --exit-status \
    --arg image $OBSERVABILITY_CHARACTERIZATION_IMAGE '
    [split("\n")[] | fromjson?] |
    [.[] | select(.["event.name"] == "http.request.completed" and
                  .["medtracker.request.id"] == "observability-characterization-request")] as $requests |
    ($requests | length) == 1 and
    ($requests[0]["event.dataset"] == "medtracker.request") and
    ($requests[0]["service.environment"] == "production") and
    ($requests[0]["service.version"] == $image) and
    ($requests[0]["http.route"] == "/login") and
    ($requests[0]["http.response.status_code"] == 200) and
    ($requests[0]["trace.id"] == "4bf92f3577b34da6a3ce929d0e0e4736") and
    ($requests[0]["event.duration"] >= 0) and
    ($requests[0]["event.id"] | test("^[0-9a-f-]{36}$"))
' $OBSERVABILITY_CHARACTERIZATION_TMP/app.log >/dev/null
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    echo 'Canonical request count or deployment identity is invalid' >&2
    exit 1
end

jq --raw-input --slurp --exit-status \
    --arg image $OBSERVABILITY_CHARACTERIZATION_IMAGE '
    [split("\n")[] | fromjson?] |
    all(.[]; .["event.name"] != "http.request.completed" or .["http.route"] != "/up")
' $OBSERVABILITY_CHARACTERIZATION_TMP/app.log >/dev/null
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    echo 'Routine health-check output was not suppressed' >&2
    exit 1
end

if string match -rq '"msg":"Request"' <$OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    echo 'Thruster request output was not disabled' >&2
    exit 1
end

jq --raw-input --slurp --exit-status '
    [split("\n")[] | fromjson?] |
    any(.[]; .["event.dataset"] == "medtracker.puma" and
             .["event.name"] == "process.message" and
             .["log.level"] != null)
' $OBSERVABILITY_CHARACTERIZATION_TMP/app.log >/dev/null
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    echo 'Production Puma output is not producer-scoped' >&2
    exit 1
end

jq --raw-input --slurp --exit-status \
    --arg image $OBSERVABILITY_CHARACTERIZATION_IMAGE '
    [split("\n")[] | fromjson?] |
    [.[] | select(.["event.dataset"] == "medtracker.job" and
                  .["medtracker.job.class"] == "ScheduleDailyRemindersJob")] as $jobs |
    ($jobs | length) == 1 and
    ($jobs[0]["event.name"] == "job.completed") and
    ($jobs[0]["event.outcome"] == "success") and
    ($jobs[0]["log.level"] == "info") and
    ($jobs[0]["service.environment"] == "production") and
    ($jobs[0]["service.version"] == $image)
' $OBSERVABILITY_CHARACTERIZATION_TMP/worker.log >/dev/null
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/worker.log
    echo 'Canonical job count or deployment identity is invalid' >&2
    exit 1
end

jq --raw-input --slurp --exit-status '
    [split("\n")[] | fromjson? |
      select(.["event.dataset"] == "medtracker.request" or .["event.dataset"] == "medtracker.job")] |
    all(.[]; (.message | type) != "object" and (.message | type) != "array")
' $OBSERVABILITY_CHARACTERIZATION_TMP/app.log \
    $OBSERVABILITY_CHARACTERIZATION_TMP/worker.log >/dev/null
or begin
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/app.log
    cat $OBSERVABILITY_CHARACTERIZATION_TMP/worker.log
    echo 'Canonical records contain nested JSON messages' >&2
    exit 1
end

echo 'Production image observability characterization passed'
