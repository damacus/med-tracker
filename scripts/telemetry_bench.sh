#!/bin/sh
# Manual telemetry bench (not a CI gate): p50/p95 request latency for the
# release binary with telemetry export off vs OTLP export to the local
# opt-in VictoriaTraces compose profile. Budget: p95 increase <= max(1ms, 5%).
set -eu
REQUESTS="${1:-300}"
PORT="${BENCH_PORT:-5515}"
PROJECT=medtracker-telemetry
DB_PORT=$(docker compose -p "$PROJECT" port postgres 5432 | cut -d: -f2)
VT_PORT=$(docker compose -p "$PROJECT" port victoriatraces 10428 | cut -d: -f2)
export DATABASE_URL="postgres://medtracker:medtracker_password@127.0.0.1:$DB_PORT/medtracker_loco"
export MEDTRACKER_PUBLIC_ORIGIN="http://127.0.0.1:$PORT"

bench() {
    label="$1"; shift
    env "$@" ./target/release/med-tracker start --environment development >"/tmp/bench-$label.log" 2>&1 &
    pid=$!
    i=0; while ! curl -fsS -o /dev/null "http://127.0.0.1:$PORT/_health" 2>/dev/null; do
        i=$((i+1)); [ "$i" -gt 120 ] && { cat "/tmp/bench-$label.log"; kill "$pid" 2>/dev/null; exit 1; }; sleep 0.5
    done
    for _ in $(seq 1 20); do curl -fsS -o /dev/null "http://127.0.0.1:$PORT/_health"; done
    : > "/tmp/bench-$label.times"
    for _ in $(seq 1 "$REQUESTS"); do
        curl -fsS -o /dev/null -w '%{time_total}\n' "http://127.0.0.1:$PORT/_health" >> "/tmp/bench-$label.times"
    done
    kill -INT "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    awk '{a[NR]=$1} END{n=NR; asort(a); printf "%s p50=%.2fms p95=%.2fms\n", "'"$label"'", a[int(n*0.5)+1]*1000, a[int(n*0.95)+1]*1000}' "/tmp/bench-$label.times"
}

bench off OTEL_EXPORTER_OTLP_ENDPOINT=
bench on "OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:$VT_PORT"
