#!/usr/bin/env fish
# Temporary, smoke-only diagnostic. Never print or upload plaintext perf output.
umask 077
function validate_events
    node -e '
const fs = require("node:fs");
const owned = new Set(process.argv[1].split(","));
let records = 0;
for (const line of fs.readFileSync(0, "utf8").split("\n")) {
  if (!line.includes("PERF_RECORD_")) continue;
  records++;
  let match = line.match(/PERF_RECORD_COMM(?: exec)?: .*:(\d+)\/(\d+)\s*$/);
  if (match) { if (!owned.has(match[1]) || !owned.has(match[2])) process.exit(1); continue; }
  match = line.match(/PERF_RECORD_(?:FORK|EXIT)\((\d+):(\d+)\):\((\d+):(\d+)\)\s*$/);
  if (match) { if (!match.slice(1).every(id => owned.has(id))) process.exit(1); continue; }
  match = line.match(/PERF_RECORD_MMAP2?\s+(-?\d+)\/(\d+):.*$/);
  if (match) {
    if (match[1] === "-1" && match[2] === "0" && /\[kernel\.kallsyms\]\s*$/.test(line)) continue;
    if (!owned.has(match[1]) || !owned.has(match[2])) process.exit(1);
    continue;
  }
  process.exit(1);
}
if (records === 0) process.exit(1);
' "$argv[1]"
end
if test "$argv[1]" = --validator-selftest
    printf 'PERF_RECORD_COMM: postgres:1/1\nPERF_RECORD_MMAP 2/2: [0x0] x /postgres\nPERF_RECORD_FORK(2:2):(1:1)\n' | validate_events 1,2; or exit 1
    if printf 'PERF_RECORD_COMM: intruder:3/3\n' | validate_events 1,2; exit 1; end
    if printf 'PERF_RECORD_MMAP 3/3: [0x0] x /intruder\n' | validate_events 1,2; exit 1; end
    if printf 'PERF_RECORD_FORK(3:3):(1:1)\n' | validate_events 1,2; exit 1; end
    printf 'PERF_RECORD_MMAP -1/0: [0x0] x [kernel.kallsyms]\n' | validate_events 1,2; or exit 1
    if printf 'PERF_RECORD_MMAP -1/0: [0x0] x /intruder\n' | validate_events 1,2; exit 1; end
    exit 0
end
test "$GITHUB_ACTIONS" = true; or exit 1
string match -rq '^[0-9a-f]{64}$' "$PG_DIAGNOSTIC_CONTAINER"; or exit 1
test (docker inspect --format '{{.Config.Image}}' "$PG_DIAGNOSTIC_CONTAINER") = postgres:18-alpine; or exit 1
set -g out "$RUNNER_TEMP/pg-pid-perf-smoke"
mkdir -m 700 "$out"; or exit 1
set -g raw "$out/private"
mkdir -m 700 "$raw"; or exit 1
set -g service "$PG_DIAGNOSTIC_CONTAINER"
set -g phase setup
set -g reason initial
set -g initial_count -1
set -g a_count -1
set -g b_count -1
set -g snapshot_success 0
set -g recorder_exit -1
set -g duration_ms -1
set -g decoder_exit -1
set -g decoder_stderr_bytes -1
set -g decoder_stderr_truncated -1
set -g summary 'samples=-1 malformed=-1 unknown=-1 outside=-1 seen_a=-1 seen_b=-1 in_window=-1'
function cleanup --on-event fish_exit
    if test -f "$raw/paused"
        docker unpause "$service" >/dev/null 2>&1
        rm -f "$raw/paused"
    end
    if set -q recorder
        set -l current_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$recorder/status" 2>/dev/null)
        set -l current_start (sudo awk '{print $22}' "/proc/$recorder/stat" 2>/dev/null)
        if set -q recorder_start; and test "$current_parent" = "$fish_pid"; and test "$current_start" = "$recorder_start"
            sudo pkill -INT -P "$recorder" >/dev/null 2>&1
            sudo kill -INT "$recorder" >/dev/null 2>&1
        end
        wait "$recorder" >/dev/null 2>&1
    end
    if set -q outside
        kill "$outside" >/dev/null 2>&1
        wait "$outside" >/dev/null 2>&1
    end
    for job in bench_a bench_b
        if set -q $job
            wait $$job >/dev/null 2>&1
        end
    end
    printf 'phase=%s reason=%s initial_count=%s a_count=%s b_count=%s snapshot_success=%s recorder_exit=%s duration_ms=%s decoder_exit=%s decoder_stderr_bytes=%s decoder_stderr_truncated=%s %s\n' "$phase" "$reason" "$initial_count" "$a_count" "$b_count" "$snapshot_success" "$recorder_exit" "$duration_ms" "$decoder_exit" "$decoder_stderr_bytes" "$decoder_stderr_truncated" "$summary" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/status.age.partial" 2>/dev/null
    set -l encryption_status $pipestatus
    if test $encryption_status[1] -eq 0; and test $encryption_status[2] -eq 0; and test -s "$out/status.age.partial"
        mv "$out/status.age.partial" "$out/status.age"
    end
    find "$out" -maxdepth 1 -name '*.partial' -delete
    sudo rm -rf -- "$raw"
end
function interrupt --on-signal SIGINT
    exit 1
end
function terminate --on-signal SIGTERM
    exit 1
end
function encrypt_file
    age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/$argv[2].age.partial" "$argv[1]" >"$raw/age.log" 2>&1; or return 1
    test -s "$out/$argv[2].age.partial"; or return 1
    mv "$out/$argv[2].age.partial" "$out/$argv[2].age"
end
set -g postmaster (docker inspect --format '{{.State.Pid}}' "$service"); or exit 1
string match -rq '^[1-9][0-9]*$' "$postmaster"; or exit 1
set -l cg (string replace -r '^0::' '' -- (cat "/proc/$postmaster/cgroup")); or exit 1
string match -q "*$service*" "$cg"; or exit 1
set -g cgdir "/sys/fs/cgroup$cg"
test -f "$cgdir/cgroup.procs"; or exit 1
set -g exe (sudo stat -Lc '%d:%i' "/proc/$postmaster/exe"); or exit 1
function pg_pids
    for p in (cat "$cgdir/cgroup.procs")
        set -l this_exe (sudo stat -Lc '%d:%i' "/proc/$p/exe" 2>/dev/null); or return 1
        test "$this_exe" = "$exe"; or continue
        set -l ancestor "$p"
        for depth in (seq 1 32)
            if test "$ancestor" = "$postmaster"
                echo "$p"
                break
            end
            set ancestor (sudo awk '/^PPid:/ {print $2}' "/proc/$ancestor/status" 2>/dev/null); or break
            string match -rq '^[1-9][0-9]*$' "$ancestor"; or break
        end
    end
end
function process_snapshot
    set -l members (pg_pids); or return 1
    set -l rows
    for p in $members
        set -l statline (sudo cat "/proc/$p/stat" 2>/dev/null); or return 1
        set -l rest (string replace -r '^.*\) ' '' -- "$statline")
        set -l fields (string split ' ' -- "$rest")
        set -l ns (sudo awk '/^NSpid:/ {print $NF}' "/proc/$p/status"); or return 1
        set -a rows "$p $fields[2] $fields[20] $ns"
    end
    printf '%s\n' $rows | sort -n
end
for i in (seq 1 60)
    docker exec "$service" pg_isready -U medtracker -d postgres >/dev/null 2>&1; and break
    test "$i" -lt 60; or exit 1
    sleep 1
end
docker exec "$service" createdb -U medtracker perf_probe; or exit 1
docker exec "$service" pgbench -U medtracker -i -s 1 perf_probe >"$raw/init.log" 2>&1; or exit 1
set phase attach
sudo perf record --dry-run -e cpu-clock -p "$postmaster" -F 49 --strict-freq --call-graph dwarf,4096 --max-size 256M --clockid mono >"$raw/dry.log" 2>&1; or exit 1
touch "$raw/paused"; or exit 1
fish --no-config -c 'sleep 20; if test -f "$argv[1]"; docker unpause "$argv[2]" >/dev/null 2>&1; end' "$raw/paused" "$service" &
docker pause "$service" >/dev/null; or exit 1
set reason snapshot_before
process_snapshot >"$raw/before"; or exit 1
set snapshot_success (math $snapshot_success + 1)
set -l pids (awk '{print $1}' "$raw/before")
set initial_count (count $pids)
contains -- "$postmaster" $pids; or exit 1
set -l pidlist (string join ',' $pids)
sudo perf record -e cpu-clock -p "$pidlist" -F 49 --strict-freq --call-graph dwarf,4096 --max-size 256M --clockid mono --timestamp-boundary --no-buildid-cache -o "$raw/perf.data" -- sleep 15 >"$raw/perf.log" 2>&1 &
set -g recorder $last_pid
set -g recorder_start (sudo awk '{print $22}' "/proc/$recorder/stat"); or exit 1
set -l perf_pid
for attempt in (seq 1 80)
    for p in (pgrep -P "$recorder")
        string match -rq '^perf' -- (ps -o comm= -p "$p" | string trim); and set perf_pid "$p"
    end
    if test -n "$perf_pid"
        set -l count_events (sudo find "/proc/$perf_pid/fd" -type l -exec readlink '{}' ';' 2>/dev/null | string match -r 'anon_inode:\[perf_event\]' | count)
        if test "$count_events" -ge (count $pids); break; end
    end
    kill -0 "$recorder" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
set reason snapshot_after
process_snapshot >"$raw/after"; or exit 1
set snapshot_success (math $snapshot_success + 1)
cmp -s "$raw/before" "$raw/after"; or exit 1
docker unpause "$service" >/dev/null; or exit 1
rm "$raw/paused"; or exit 1
set phase workload
set reason clock_start
set -l window_start (node -e 'console.log((Number(process.hrtime.bigint()) / 1e9).toFixed(9))'); or exit 1
set reason utc_start
set -l utc_start (date -u +%FT%TZ)
set reason cpu_before
cat "$cgdir/cpu.stat" >"$raw/cpu-before"
yes >/dev/null &
set -g outside $last_pid
set reason outside_start
set -l outside_start (awk '{print $14 + $15}' "/proc/$outside/stat"); or exit 1
docker exec "$service" pgbench -U medtracker -T 15 -c 1 -j 1 perf_probe >"$raw/bench-a.log" 2>&1 &
set -g bench_a $last_pid
sleep 2
set reason snapshot_a
process_snapshot >"$raw/a"; or exit 1
set snapshot_success (math $snapshot_success + 1)
set -l a (comm -13 (awk '{print $1}' "$raw/before" | sort | psub) (awk '{print $1}' "$raw/a" | sort | psub))
set a_count (count $a)
set reason new_a_count
test (count $a) -eq 1; or exit 1
docker exec "$service" pgbench -U medtracker -T 12 -c 1 -j 1 perf_probe >"$raw/bench-b.log" 2>&1 &
set -g bench_b $last_pid
sleep 2
set reason snapshot_b
process_snapshot >"$raw/b"; or exit 1
set snapshot_success (math $snapshot_success + 1)
set -l b (comm -13 (awk '{print $1}' "$raw/a" | sort | psub) (awk '{print $1}' "$raw/b" | sort | psub))
set b_count (count $b)
set reason new_b_count
test (count $b) -eq 1; or exit 1
sleep 3
set reason snapshot_end
process_snapshot >"$raw/end"; or exit 1
set snapshot_success (math $snapshot_success + 1)
set reason a_identity
test (awk -v id="$a" '$1 == id {print $3}' "$raw/a") = (awk -v id="$a" '$1 == id {print $3}' "$raw/end"); or exit 1
set reason b_identity
test (awk -v id="$b" '$1 == id {print $3}' "$raw/b") = (awk -v id="$b" '$1 == id {print $3}' "$raw/end"); or exit 1
set phase validate-samples
set reason recorder_wait
wait "$recorder"
set recorder_exit $status
test "$recorder_exit" -eq 0; or exit 1
set -e recorder
set reason clock_end
set -l window_end (node -e 'console.log((Number(process.hrtime.bigint()) / 1e9).toFixed(9))'); or exit 1
set -l utc_end (date -u +%FT%TZ)
set reason cpu_after
cat "$cgdir/cpu.stat" >"$raw/cpu-after"
set duration_ms (awk -v start="$window_start" -v end="$window_end" 'BEGIN {printf "%.0f", (end-start)*1000}')
set reason recording_duration
awk -v start="$window_start" -v end="$window_end" 'BEGIN {if (end - start < 10) exit 1}'; or exit 1
set reason outside_end
set -l outside_end (awk '{print $14 + $15}' "/proc/$outside/stat"); or exit 1
set reason outside_inactive
test "$outside_end" -gt "$outside_start"; or exit 1
set reason decode_samples
sudo perf script -i "$raw/perf.data" --hide-call-graph -F pid,time >"$raw/samples" 2>"$raw/script.log"
set decoder_exit $status
if test "$decoder_exit" -ne 0
    set decoder_stderr_bytes (wc -c <"$raw/script.log" | string trim)
    set decoder_stderr_truncated 0
    if test "$decoder_stderr_bytes" -gt 16384
        set decoder_stderr_truncated 1
    end
    head -c 16384 "$raw/script.log" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/decoder-stderr.age.partial" 2>/dev/null
    set -l encryption_status $pipestatus
    if test $encryption_status[1] -eq 0; and test $encryption_status[2] -eq 0; and test -s "$out/decoder-stderr.age.partial"
        mv "$out/decoder-stderr.age.partial" "$out/decoder-stderr.age"
    end
    exit 1
end
set reason empty_samples
set summary 'samples=0 malformed=0 unknown=0 outside=0 seen_a=0 seen_b=0 in_window=0'
test -s "$raw/samples"; or exit 1
set summary (awk -v a="$a" -v b="$b" -v x="$outside" -v initial="$pidlist" -v lo="$window_start" -v hi="$window_end" '
  BEGIN {n=split(initial,p,","); for(i=1;i<=n;i++) owned[p[i]]=1; owned[a]=1; owned[b]=1}
  NF {count++; gsub(":","",$2); if($1 !~ /^[0-9]+$/ || $2 !~ /^[0-9.]+$/) {malformed++; next} if(!owned[$1]) unknown++; if($1 == x) outside++; if($1 == a) seen_a=1; if($1 == b) seen_b=1; if($2>=lo && $2<=hi) in_window=1}
  END {printf "samples=%d malformed=%d unknown=%d outside=%d seen_a=%d seen_b=%d in_window=%d",count,malformed,unknown,outside,seen_a,seen_b,in_window}
' "$raw/samples")
set reason sample_validation
awk -v a="$a" -v b="$b" -v x="$outside" -v initial="$pidlist" -v lo="$window_start" -v hi="$window_end" '
  BEGIN { n=split(initial, p, ","); for (i=1; i<=n; i++) owned[p[i]]=1; owned[a]=1; owned[b]=1 }
  NF { gsub(":", "", $2); if ($1 !~ /^[0-9]+$/ || $2 !~ /^[0-9.]+$/ || $1 == x || !owned[$1]) exit 1; if ($1 == a) seen_a=1; if ($1 == b) seen_b=1; if ($2 >= lo && $2 <= hi) in_window=1 }
  END { if (!seen_a || !seen_b || !in_window) exit 1 }
' "$raw/samples"; or exit 1
set phase validate-events
sudo perf script -i "$raw/perf.data" --show-task-events --show-mmap-events >"$raw/events" 2>"$raw/events.log"; or exit 1
validate_events "$pidlist,$a,$b" <"$raw/events"; or exit 1
rg -q 'FORK' "$raw/events"; or exit 1
rg -q 'COMM' "$raw/events"; or exit 1
rg -q 'MMAP' "$raw/events"; or exit 1
set phase validate-symbols
sudo perf report --stdio --no-children --symfs "/proc/$postmaster/root" -i "$raw/perf.data" >"$raw/report" 2>"$raw/report.log"; or exit 1
rg -q 'postgres.*\[\.\][[:space:]]+(Exec|PortalRun|PostgresMain|standard_planner|FunctionCall)' "$raw/report"; or exit 1
rg -q '\[kernel\.kallsyms\].*\[k\][[:space:]]+[[:alnum:]_]' "$raw/report"; or exit 1
cat "$raw/before" "$raw/a" "$raw/b" "$raw/end" >"$raw/process-map"
printf 'clock=monotonic start=%s end=%s utc_start=%s utc_end=%s bytes=%s samples=%s\n' "$window_start" "$window_end" "$utc_start" "$utc_end" (stat -c %s "$raw/perf.data") (wc -l <"$raw/samples" | string trim) >"$raw/window"
cat "$raw/cpu-before" "$raw/cpu-after" >>"$raw/window"
set phase encrypt
set reason data_ownership
sudo chown (id -u):(id -g) "$raw/perf.data"; or exit 1
for name in perf.data events report process-map window samples
    encrypt_file "$raw/$name" "$name"; or exit 1
end
wait "$bench_a"; or exit 1
set -e bench_a
wait "$bench_b"; or exit 1
set -e bench_b
docker exec "$service" dropdb -U medtracker perf_probe; or exit 1
set phase complete
echo 'Smoke artefacts encrypted; task/MMAP lineage requires independent review.'
