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
  const match = line.match(/^\s*(\d+(?:\.\d+)?)%?\s*;\s*(.*?)\s*$/);
  const value = match ? Number(match[1]) : NaN;
  if (!match || !Number.isFinite(value) || value < 0 || value > 100) {
    if (!malformed) { firstSemicolons = (line.match(/;/g) || []).length; firstFields = firstSemicolons + 1; }
    malformed = true; continue;
  }
  const prefix = match[2].match(/^\[([.k])\]\s*/);
  const mode = prefix ? (prefix[1] === "." ? "user" : "kernel") : "unknown";
  if (mode === "unknown") unknownMode++;
  const label = match[2].replace(/^\[[.k]\]\s*/, "");
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
    set -l result (printf '20.00;[.] ExecScan\n30.00;[k] schedule\n10.00;[.] 0x1234\n' | sanitise_histogram); or exit 1
    string match -q '*"symbol":"ExecScan"*"symbol":"[unknown]"*' "$result"; or exit 1
    string match -q '*"mode":"user"*"mode":"kernel"*' "$result"; or exit 1
    if printf '20.00;[.] 0x1234\n30.00;[k] 0x1234\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '20.00;[.] ExecScan\n' | sanitise_histogram >/dev/null; exit 1; end
    if printf '30.00;[k] schedule\n' | sanitise_histogram >/dev/null; exit 1; end
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
set -g report_exit -1
set -g report_stdout_bytes -1
set -g report_stdout_truncated -1
set -g report_stderr_bytes -1
set -g report_stderr_truncated -1
set -g symfs_realpath_is_root -1
set -g pg_binary_readable -1
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
    if set -q outside
        kill "$outside" >/dev/null 2>&1
        wait "$outside" >/dev/null 2>&1
    end
    for job in bench_a bench_b
        if set -q $job
            wait $$job >/dev/null 2>&1
        end
    end
    printf 'phase=%s reason=%s initial_count=%s a_count=%s b_count=%s snapshot_success=%s recorder_exit=%s duration_ms=%s report_exit=%s report_stdout_bytes=%s report_stdout_truncated=%s report_stderr_bytes=%s report_stderr_truncated=%s symfs_realpath_is_root=%s pg_binary_readable=%s %s %s\n' "$phase" "$reason" "$initial_count" "$a_count" "$b_count" "$snapshot_success" "$recorder_exit" "$duration_ms" "$report_exit" "$report_stdout_bytes" "$report_stdout_truncated" "$report_stderr_bytes" "$report_stderr_truncated" "$symfs_realpath_is_root" "$pg_binary_readable" "$report_info" "$summary" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/status.age.partial" 2>/dev/null
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
set -l symfs_resolved (sudo realpath "/proc/$postmaster/root" 2>/dev/null)
if test $status -eq 0
    set symfs_realpath_is_root 0
    if test "$symfs_resolved" = /
        set symfs_realpath_is_root 1
    end
end
set pg_binary_readable 0
if sudo test -r "/proc/$postmaster/root/usr/local/bin/postgres"
    set pg_binary_readable 1
end
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
set -l scope_start (node -e 'console.log((Number(process.hrtime.bigint()) / 1e9).toFixed(9))'); or exit 1
sleep 3
kill -0 "$recorder" 2>/dev/null; or exit 1
set -l scope_end (node -e 'console.log((Number(process.hrtime.bigint()) / 1e9).toFixed(9))'); or exit 1
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
set reason scope_selection
set -l stable (comm -12 (sort "$raw/b" | psub) (sort "$raw/end" | psub))
set -l known
for line in $stable
    set -l fields (string split ' ' -- "$line")
    set -a known "$fields[1]"
end
contains -- "$a" $known; and contains -- "$b" $known; or exit 1
set -l known_list (string join ',' $known)
set -l interval "$scope_start,$scope_end"
set phase coverage
set reason decode_samples
sudo perf script -i "$raw/perf.data" --hide-call-graph -F pid,time,period >"$raw/samples" 2>"$raw/script.log"; or exit 1
node -e '
const fs=require("node:fs"), [path, ids, aa, bb, outside, lo, hi] = process.argv.slice(1);
const known=new Set(ids.split(",")); let total=0, selected=0, excluded=0, a=0, b=0, out=0, totalPeriod=0n, selectedPeriod=0n;
for(const line of fs.readFileSync(path,"utf8").split("\n")) {
  if(!line.trim()) continue;
  const m=line.trim().match(/^(\d+)\s+(\d+(?:\.\d+)?):?\s+(\d+)$/); if(!m) process.exit(1);
  const t=Number(m[2]); if(t < Number(lo) || t > Number(hi)) continue;
  const p=BigInt(m[3]); total++; totalPeriod+=p; if(m[1]===outside) out++;
  if(known.has(m[1])) { selected++; selectedPeriod+=p; } else excluded++;
  if(m[1]===aa) a++; if(m[1]===bb) b++;
}
if(!total || !selected || !a || !b || out) process.exit(1);
console.log(JSON.stringify({total,selected,excluded,a,b,out,total_period:totalPeriod.toString(),selected_period:selectedPeriod.toString()}));
' "$raw/samples" "$known_list" "$a" "$b" "$outside" "$scope_start" "$scope_end" >"$raw/coverage" 2>"$raw/coverage.log"; or exit 1
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
verify_filter_reports "$raw" "$known_list" "$a" "$b" "$missing" >"$raw/proof-check.log" 2>&1; or exit 1
set phase aggregate
set reason report_generate
sudo perf report --stdio --no-children --call-graph none --percentage relative --sort symbol --fields overhead,symbol --field-separator ';' --pid "$known_list" --time "$interval" --symfs "/proc/$postmaster/root" -i "$raw/perf.data" >"$raw/report" 2>"$raw/report.log"
set report_exit $status
set reason report_debug_encrypt
set report_stdout_bytes (wc -c <"$raw/report" | string trim)
set report_stdout_truncated 0
if test "$report_stdout_bytes" -gt 32768
    set report_stdout_truncated 1
end
set report_stderr_bytes (wc -c <"$raw/report.log" | string trim)
set report_stderr_truncated 0
if test "$report_stderr_bytes" -gt 16384
    set report_stderr_truncated 1
end
head -c 32768 "$raw/report" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/unsanitised-report-stdout.age.partial" 2>/dev/null
set -l stdout_encryption_status $pipestatus
test $stdout_encryption_status[1] -eq 0; and test $stdout_encryption_status[2] -eq 0; and test -s "$out/unsanitised-report-stdout.age.partial"; or exit 1
mv "$out/unsanitised-report-stdout.age.partial" "$out/unsanitised-report-stdout.age"; or exit 1
head -c 16384 "$raw/report.log" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/unsanitised-report-stderr.age.partial" 2>/dev/null
set -l stderr_encryption_status $pipestatus
test $stderr_encryption_status[1] -eq 0; and test $stderr_encryption_status[2] -eq 0; and test -s "$out/unsanitised-report-stderr.age.partial"; or exit 1
mv "$out/unsanitised-report-stderr.age.partial" "$out/unsanitised-report-stderr.age"; or exit 1
set reason report_generate
test "$report_exit" -eq 0; or exit 1
set reason report_sanitise
sanitise_histogram "$raw/report-diagnostic" <"$raw/report" >"$raw/safe-report" 2>"$raw/sanitise.log"
set -l sanitise_exit $status
set report_info (node -e 'const d=JSON.parse(require("node:fs").readFileSync(process.argv[1],"utf8")); const keys=["rows","pg_user","kernel","unknown_mode","redacted","first_semicolons","first_fields"]; if(!["none","malformed_row","no_rows","missing_pg_user","missing_kernel"].includes(d.reason)||keys.some(k=>!Number.isSafeInteger(d[k])||d[k]<-1))process.exit(1); console.log(`sanitiser=${d.reason} `+keys.map(k=>`${k}=${d[k]}`).join(" "))' "$raw/report-diagnostic" 2>"$raw/report-diagnostic.log"); or exit 1
test "$sanitise_exit" -eq 0; or exit 1
set reason report_size
test (wc -c <"$raw/safe-report" | string trim) -le 1048576; or exit 1
set -l bytes (stat -c %s "$raw/perf.data")
test "$bytes" -lt 268435456; or exit 1
set summary (node -e 'const fs=require("node:fs"); const x=JSON.parse(fs.readFileSync(process.argv[1],"utf8")), y=JSON.parse(fs.readFileSync(process.argv[2],"utf8")); console.log(`scope=verified_pid_subset clock=monotonic start=${process.argv[4]} end=${process.argv[5]} total_samples=${x.total} known_samples=${x.selected} excluded_samples=${x.excluded} total_period=${x.total_period} known_period=${x.selected_period} lost_records=0 redacted_symbol_rows=${y.redacted_rows} redacted_percent=${y.redacted_percent} raw_bytes=${process.argv[3]} raw_cap_bytes=268435456`)' "$raw/coverage" "$raw/safe-report" "$bytes" "$scope_start" "$scope_end")
set phase encrypt
set reason aggregate_encrypt
encrypt_file "$raw/safe-report" aggregate-report; or exit 1
wait "$bench_a"; or exit 1
set -e bench_a
wait "$bench_b"; or exit 1
set -e bench_b
docker exec "$service" dropdb -U medtracker perf_probe; or exit 1
set phase complete
echo 'Encrypted subset aggregate and coverage status created.'
