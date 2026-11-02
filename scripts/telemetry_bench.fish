#!/usr/bin/env fish
# Manual telemetry bench (not a CI gate): p50/p95 request latency for the
# release binary with telemetry export off vs OTLP export to the local
# opt-in VictoriaTraces compose profile. Budget: p95 increase <= max(1ms, 5%).
set -g requests 300
if test (count $argv) -gt 0
    set requests $argv[1]
end
set -g port 5150
set -q BENCH_PORT; and set port $BENCH_PORT
set -l project medtracker-telemetry
set -g db_port (docker compose -p $project port postgres 5432 | cut -d: -f2 | tr -d '[:space:]')
set -g vt_port (docker compose -p $project port victoriatraces 10428 | cut -d: -f2 | tr -d '[:space:]')
set -x DATABASE_URL "postgres://medtracker:medtracker_password@127.0.0.1:$db_port/medtracker_loco"
set -x MEDTRACKER_PUBLIC_ORIGIN "http://127.0.0.1:$port"

./target/release/med-tracker db migrate --environment development >/tmp/bench-migrate.log 2>&1
or begin
    cat /tmp/bench-migrate.log
    exit 1
end

# VictoriaTraces is distroless so compose --wait cannot health-check it;
# poll its HTTP health endpoint ourselves before benchmarking.
set -l vt_ready 0
for i in (seq 60)
    curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:$vt_port/health" 2>/dev/null; and set vt_ready 1; and break
    sleep 0.5
end
if test $vt_ready -ne 1
    echo "victoriatraces did not become healthy"
    exit 1
end

function bench --argument-names label
    set -l logfile /tmp/bench-$label.log
    set -l times /tmp/bench-$label.times
    /usr/bin/env $argv[2..-1] ./target/release/med-tracker start --environment development >$logfile 2>&1 &
    set -l pid (pgrep -nf "med-tracker start --environment development")
    set -l ready 0
    for i in (seq 120)
        curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:$port/_health" 2>/dev/null; and set ready 1; and break
        sleep 0.5
    end
    if test $ready -ne 1
        cat $logfile
        kill -INT $pid 2>/dev/null
        exit 1
    end
    for i in (seq 20)
        curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:$port/_health" >/dev/null 2>&1
    end
    printf "" > $times
    for i in (seq $requests)
        curl -fsS --max-time 5 -o /dev/null -w '%{time_total}\n' "http://127.0.0.1:$port/_health" >> $times 2>/dev/null
    end
    kill -INT $pid 2>/dev/null
    wait $pid
    sort -n $times | awk -v label=$label '{a[NR]=$1} END{n=NR; printf "%s p50=%.2fms p95=%.2fms\n", label, a[int(n*0.5)+1]*1000, a[int(n*0.95)+1]*1000}'
end

bench off OTEL_EXPORTER_OTLP_ENDPOINT=
bench on "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://127.0.0.1:$vt_port/insert/opentelemetry/v1/traces"
