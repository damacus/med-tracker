#!/usr/bin/env fish
# Temporary, smoke-only diagnostic. Never print or upload plaintext perf output.
umask 077
function sanitise_histogram
    node -e '
const fs = require("node:fs");
const rows = [];
let redacted = 0;
let redactedPercent = 0;
let pgUser = 0, kernel = 0, unknownMode = 0;
let malformed = false, firstSemicolons = -1, firstFields = -1;
for (const line of fs.readFileSync(0, "utf8").split("\n")) {
  if (!line.trim() || line.trimStart().startsWith("#")) continue;
  const match = line.match(/^\s*(\d+(?:\.\d+)?)%?\s*;\s*([^;]*?)(?:;\s*-\s+-\s*)?\s*$/);
  const value = match ? Number(match[1]) : NaN;
  if (!match || !Number.isFinite(value) || value < 0 || value > 100) {
    if (!malformed) { firstSemicolons = (line.match(/;/g) || []).length; firstFields = firstSemicolons + 1; }
    malformed = true; continue;
  }
  const prefix = match[2].match(/^\[([.k])\]\s*/);
  const mode = prefix ? (prefix[1] === "." ? "user" : "kernel") : "unknown";
  if (mode === "unknown") unknownMode++;
  const label = match[2].replace(/^\[[.k]\]\s*/, "").trim();
  const safe = /^[A-Za-z_][A-Za-z0-9_.]*$/.test(label) && !/0x[0-9a-f]/i.test(label);
  if (!safe) { redacted++; redactedPercent += value; }
  if (safe && mode === "user" && /^(Exec|PortalRun|PostgresMain|standard_planner|FunctionCall)/.test(label)) pgUser++;
  if (safe && mode === "kernel") kernel++;
  rows.push({percent:value, mode, symbol:safe ? label : "[unknown]"});
}
const reason = malformed ? "malformed_row" : !rows.length ? "no_rows" : !pgUser ? "missing_pg_user" : !kernel ? "missing_kernel" : "none";
if (process.argv[1]) fs.writeFileSync(process.argv[1], JSON.stringify({reason, rows:rows.length, pg_user:pgUser, kernel, unknown_mode:unknownMode, redacted, first_semicolons:firstSemicolons, first_fields:firstFields}));
if (reason !== "none") process.exit(1);
console.log(JSON.stringify({scope:"verified_pid_subset", redacted_rows:redacted, redacted_percent:redactedPercent, rows}));
' $argv
end
function verify_filter_reports
    node -e '
const fs=require("node:fs"), [dir, ids, aa, bb, missing] = process.argv.slice(1), coverage=JSON.parse(fs.readFileSync(`${dir}/coverage`,"utf8"));
function check(name, allowed, expected) {
  let count=0, rows=0;
  for(const line of fs.readFileSync(`${dir}/proof-${name}`,"utf8").split("\n")) {
    if(!line.trim() || line.trimStart().startsWith("#")) continue;
    const m=line.match(/^\s*(\d+)\s*;\s*(\d+):[^;]*\s*$/); if(!m || !allowed.has(m[2])) process.exit(1);
    count+=Number(m[1]); rows++;
  }
  if(count!==expected || (expected>0 && !rows) || (expected===0 && rows)) process.exit(1);
}
if(!coverage.a || !coverage.b) process.exit(1);
check("a",new Set([aa]),coverage.a);
check("known",new Set(ids.split(",")),coverage.selected);
check("missing",new Set([missing]),0);
' $argv
end
if test "$argv[1]" = --validator-selftest
    printf ' 20.00%% ;[.] ExecScan ;-      -\n 30.00%% ;[k] schedule ;-      -\n' | sanitise_histogram >/dev/null; or exit 1
    set -l result (printf '20.00;[.] ExecScan\n30.00;[k] schedule\n10.00;[.] 0x1234\n' | sanitise_histogram); or exit 1
    string match -q '*"symbol":"ExecScan"*"symbol":"[unknown]"*' "$result"; or exit 1
    string match -q '*"mode":"user"*"mode":"kernel"*' "$result"; or exit 1
    if printf '20.00;[.] 0x1234\n30.00;[k] 0x1234\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '20.00;[.] ExecScan\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '30.00;[k] schedule\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '20.00;[.] ExecScan;private\n30.00;[k] schedule;- -\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '20.00;[.] ExecScan;- -;extra\n30.00;[k] schedule;- -\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '20.00;/private/path\nwrong\n' | sanitise_histogram >/dev/null; exit 1; end
    set -l fixture (mktemp -d); or exit 1
    printf 'bad;row;extra\n' | sanitise_histogram "$fixture/diagnostic" >/dev/null
    test $status -ne 0; and jq -e '.reason == "malformed_row" and .first_semicolons == 2 and .first_fields == 3' "$fixture/diagnostic" >/dev/null; or begin; rm -rf "$fixture"; exit 1; end
    printf '\n' | sanitise_histogram "$fixture/diagnostic" >/dev/null
    test $status -ne 0; and jq -e '.reason == "no_rows" and .rows == 0' "$fixture/diagnostic" >/dev/null; or begin; rm -rf "$fixture"; exit 1; end
    printf '20.00;[.] 0x1234\n30.00;[k] 0x1234\n' | sanitise_histogram "$fixture/diagnostic" >/dev/null
    test $status -ne 0; and jq -e '.reason == "missing_pg_user" and .redacted == 2' "$fixture/diagnostic" >/dev/null; or begin; rm -rf "$fixture"; exit 1; end
    printf '20.00;[.] ExecScan\n' | sanitise_histogram "$fixture/diagnostic" >/dev/null
    test $status -ne 0; and jq -e '.reason == "missing_kernel" and .pg_user == 1' "$fixture/diagnostic" >/dev/null; or begin; rm -rf "$fixture"; exit 1; end
    printf '{"a":2,"b":3,"selected":5}\n' >"$fixture/coverage"
    printf '2; 1:postgres\n' >"$fixture/proof-a"
    printf '2; 1:postgres\n3; 2:postgres\n' >"$fixture/proof-known"
    printf '# no samples\n' >"$fixture/proof-missing"
    verify_filter_reports "$fixture" 1,2 1 2 3; or begin; rm -rf "$fixture"; exit 1; end
    printf '2; 1:postgres\n1; 2:postgres\n' >"$fixture/proof-a"
    if verify_filter_reports "$fixture" 1,2 1 2 3; rm -rf "$fixture"; exit 1; end
    rm -rf "$fixture"
    exit 0
end
test "$GITHUB_ACTIONS" = true; or exit 1
string match -rq '^[0-9]+$' "$GITHUB_RUN_ID"; or exit 1
string match -rq '^[0-9a-f]{40}$' "$GITHUB_SHA"; or exit 1
string match -rq '^[0-9a-f]{64}$' "$PG_DIAGNOSTIC_CONTAINER"; or exit 1
test (docker inspect --format '{{.Config.Image}}' "$PG_DIAGNOSTIC_CONTAINER") = postgres:18-alpine; or exit 1
set -g out "$RUNNER_TEMP/pg-capture"
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
set -g report_exit -1
set -g report_info 'sanitiser=unavailable rows=-1 pg_user=-1 kernel=-1 unknown_mode=-1 redacted=-1 first_semicolons=-1 first_fields=-1'
set -g summary 'coverage=unavailable'
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
    if set -q sampler
        touch "$raw/sampler.stop"
        wait "$sampler" >/dev/null 2>&1
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
    printf 'run_id=%s source_sha=%s phase=%s reason=%s initial_count=%s a_count=%s b_count=%s snapshot_success=%s recorder_exit=%s duration_ms=%s report_exit=%s %s %s\n' "$GITHUB_RUN_ID" "$GITHUB_SHA" "$phase" "$reason" "$initial_count" "$a_count" "$b_count" "$snapshot_success" "$recorder_exit" "$duration_ms" "$report_exit" "$report_info" "$summary" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/status.age.partial" 2>/dev/null
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
    docker exec "$service" pg_isready -h 127.0.0.1 -U medtracker -d postgres >/dev/null 2>&1; and break
    test "$i" -lt 60; or exit 1
    sleep 1
end
test (docker inspect --format '{{.State.Pid}}' "$service") = "$postmaster"; or exit 1
set -g exe (sudo stat -Lc '%d:%i' "/proc/$postmaster/exe"); or exit 1
test "$exe" = (sudo stat -Lc '%d:%i' "/proc/$postmaster/root/usr/local/bin/postgres"); or exit 1
set phase attach
sudo perf record --dry-run -e cpu-clock -p "$postmaster" -F 49 --strict-freq --max-size 256M --clockid mono >"$raw/dry.log" 2>&1; or exit 1
touch "$raw/paused"; or exit 1
fish --no-config -c 'sleep 20; if test -f "$argv[1]"; docker unpause "$argv[2]" >/dev/null 2>&1; end' "$raw/paused" "$service" &
docker pause "$service" >/dev/null; or exit 1
process_snapshot >"$raw/before"; or exit 1
set -l pids (awk '{print $1}' "$raw/before")
contains -- "$postmaster" $pids; or exit 1
set initial_count (count $pids)
set -l pidlist (string join ',' $pids)
sudo perf record -e cpu-clock -p "$pidlist" -F 49 --strict-freq --max-size 256M --clockid mono --timestamp-boundary --no-buildid-cache -o "$raw/perf.data" >"$raw/perf.log" 2>&1 &
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
set -l perf_start (sudo awk '{print $22}' "/proc/$perf_pid/stat"); or exit 1
process_snapshot >"$raw/after"; or exit 1
cmp -s "$raw/before" "$raw/after"; or exit 1
docker unpause "$service" >/dev/null; or exit 1
rm "$raw/paused"; or exit 1
set reason sampler_start
set -l node_path (command -s node); or exit 1
sudo "$node_path" scripts/diagnostics/pg-capture-sampler.js "$raw" "$cgdir" "$postmaster" "$service" >"$raw/sampler.log" 2>&1 &
set -g sampler $last_pid
for attempt in (seq 1 80)
    test -f "$raw/sampler.ready"; and break
    kill -0 "$sampler" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
touch "$out/collector.ready"; or exit 1
set phase await_timeout
set reason wait_marker
set -l found 0
for attempt in (seq 1 7200)
    if test -f "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE"
        set -l marker (cat "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE")
        if string match -rq '^[0-9]{19}$' "$marker"
            set found 1
            break
        end
    end
    test -f "$out/test.done"; and exit 1
    kill -0 "$recorder" 2>/dev/null; and kill -0 "$sampler" 2>/dev/null; or exit 1
    sleep 0.25
end
test "$found" -eq 1; or exit 1
set phase after_timeout
set reason wait_recording
set -l remaining (node -e 'const m=BigInt(process.argv[1]), n=BigInt(Date.now())*1000000n, d=m+60000000000n-n; if(d<=0n||d>60001000000n)process.exit(1); console.log(Number(d>60000000000n?60000000000n:d)/1e9)' "$marker"); or exit 1
sleep "$remaining"; or exit 1
kill -0 "$recorder" 2>/dev/null; and kill -0 "$sampler" 2>/dev/null; or exit 1
set reason stop_recording
set -l current_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$perf_pid/status"); or exit 1
set -l current_start (sudo awk '{print $22}' "/proc/$perf_pid/stat"); or exit 1
test "$current_parent" = "$recorder"; and test "$current_start" = "$perf_start"; or exit 1
sudo kill -INT "$perf_pid"; or exit 1
wait "$recorder"; or exit 1
set -e recorder
touch "$raw/sampler.stop"; or exit 1
wait "$sampler"; or exit 1
set -e sampler
sudo chown (id -u):(id -g) "$raw/snapshots.jsonl"; or exit 1
set -l bytes (stat -c %s "$raw/perf.data")
test "$bytes" -lt 268435456; or exit 1
set phase coverage
set reason decode_samples
sudo perf script -i "$raw/perf.data" --hide-call-graph -F pid,time,period >"$raw/samples" 2>"$raw/script.log"; or exit 1
set reason select_window
node scripts/diagnostics/pg-capture-analysis.js "$raw" "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE" >"$raw/analysis.log" 2>&1; or exit 1
set -l known_list (cat "$raw/pids")
set -l a (cat "$raw/a")
set -l b (cat "$raw/b")
set -l interval (node -e 'const x=require(process.argv[1]); console.log(`${x.start},${x.end}`)' "$raw/scope.json")
set reason lost_records
sudo perf script -i "$raw/perf.data" --show-lost-events --hide-call-graph -F pid,time,period >"$raw/loss" 2>"$raw/loss.log"; or exit 1
set -l lost_records (rg -c 'PERF_RECORD_LOST(_SAMPLES)?' "$raw/loss"); or set lost_records 0
set summary "lost_records=$lost_records"
test "$lost_records" -eq 0; or exit 1
set -l missing 2147483647
test ! -e "/proc/$missing"; or exit 1
set reason pid_filter_proof
for entry in a known missing
    set -l selected_pid
    switch "$entry"
        case a; set selected_pid "$a"
        case known; set selected_pid "$known_list"
        case missing; set selected_pid "$missing"
    end
    sudo perf report --stdio --no-children --call-graph none --sort tgid --fields sample,tgid --field-separator ';' --pid "$selected_pid" --time "$interval" --symfs "/proc/$postmaster/root" -i "$raw/perf.data" >"$raw/proof-$entry" 2>"$raw/proof-$entry.log"; or exit 1
end
cp "$raw/scope.json" "$raw/coverage"; or exit 1
verify_filter_reports "$raw" "$known_list" "$a" "$b" "$missing" >"$raw/proof-check.log" 2>&1; or exit 1
set phase aggregate
set reason report_generate
sudo perf report --stdio --no-children --call-graph none --percentage relative --sort symbol --fields overhead,symbol --field-separator ';' --pid "$known_list" --time "$interval" --symfs "/proc/$postmaster/root" -i "$raw/perf.data" >"$raw/report" 2>"$raw/report.log"; or exit 1
set reason report_sanitise
sanitise_histogram "$raw/report-diagnostic" <"$raw/report" >"$raw/safe-report" 2>"$raw/sanitise.log"; or exit 1
test (wc -c <"$raw/safe-report" | string trim) -le 1048576; or exit 1
set -l clock_ticks (getconf CLK_TCK); or exit 1
string match -rq '^[1-9][0-9]*$' "$clock_ticks"; or exit 1
set summary (node -e 'const fs=require("node:fs"),x=JSON.parse(fs.readFileSync(process.argv[1],"utf8")),y=JSON.parse(fs.readFileSync(process.argv[2],"utf8")); console.log(`scope=verified_pid_subset clock=monotonic start=${x.start} end=${x.end} calibration_uncertainty_ns=${x.calibration_uncertainty_ns} cpu_observation_start_ns=${x.cpu_observation_start_ns} cpu_observation_end_ns=${x.cpu_observation_end_ns} snapshot_unverified=${x.snapshot_unverified} total_samples=${x.total} known_samples=${x.selected} excluded_samples=${x.excluded} total_period=${x.total_period} known_period=${x.selected_period} pg_usage_usec=${x.cpu.usage_usec} pg_user_usec=${x.cpu.user_usec} pg_system_usec=${x.cpu.system_usec} pg_throttled_usec=${x.cpu.throttled_usec} host_ticks_per_second=${process.argv[6]} host_jiffy_deltas=${x.host.join(",")} lost_records=0 redacted_symbol_rows=${y.redacted_rows} redacted_percent=${y.redacted_percent} raw_bytes=${process.argv[3]} raw_cap_bytes=268435456`)' "$raw/scope.json" "$raw/safe-report" "$bytes" "$scope_start" "$scope_end" "$clock_ticks")
set phase encrypt
set reason aggregate_encrypt
encrypt_file "$raw/safe-report" aggregate-report; or exit 1
set phase complete
