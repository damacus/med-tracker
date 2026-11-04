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
set -g phase setup
set -g reason initial
set -g initial_count -1
set -g a_count -1
set -g b_count -1
set -g snapshot_success 0
set -g recorder_exit -1
set -g sampler_exit -1
set -g child_shutdown_failed 0
set -g capture_children_stopped 0
set -g recorder_launched 0
set -g sampler_launched 0
set -g selftest_mode 0
set -g duration_ms -1
set -g report_exit -1
set -g report_info 'sanitiser=unavailable rows=-1 pg_user=-1 kernel=-1 unknown_mode=-1 redacted=-1 first_semicolons=-1 first_fields=-1'
set -g summary 'coverage=unavailable'
set -g child_wrapper 'const fs=require("node:fs"),os=require("node:os"),{spawn}=require("node:child_process");const [result,command,...args]=process.argv.slice(1);const child=spawn(command,args,{stdio:"inherit"});child.once("error",()=>{process.exitCode=1});child.once("exit",(code,signal)=>{const status=code===null?128+(os.constants.signals[signal]??127):code;fs.writeFileSync(result+".tmp",`${status}\n`,{mode:0o600});fs.renameSync(result+".tmp",result)})'
function bounded_wait
    set -g bounded_alive 0
    for attempt in (seq 1 "$argv[2]")
        kill -0 "$argv[1]" 2>/dev/null; or begin
            wait "$argv[1]" >/dev/null 2>&1
            return 0
        end
        sleep 0.1
    end
    set -g bounded_alive 1
    return 1
end
function child_result
    test -f "$argv[1]"; or return 1
    set -l result (cat "$argv[1]")
    string match -rq '^(0|[1-9][0-9]{0,2})$' -- "$result"; or return 1
    test "$result" -eq 0
end
function profile_result_ok
    set -l result_path "$argv[1]"
    test -f "$result_path"; or return 1
    set -l result (cat "$result_path")
    string match -rq '^(0|[1-9][0-9]{0,2})$' -- "$result"; and test "$result" -le 255; or return 1
    set -l status_path (string replace -r '\.result$' '.status' -- "$result_path")
    printf '%s\n' "$result" >"$status_path"; or return 1
    if test "$result_path" = "$raw/recorder.result"
        set -l requested 0
        if set -q recorder_requested_sigint; set requested "$recorder_requested_sigint"; end
        set -l header_ok 0
        if test -s "$raw/perf.data"; and sudo "$perf_path" report --header-only -i "$raw/perf.data" >/dev/null 2>&1
            set header_ok 1
        end
        recorder_status_allowed "$result" "$requested" "$header_ok"
        return $status
    end
    test "$result" -eq 0
end
function recorder_status_allowed
    test "$argv[2]" = 1; and test "$argv[3]" = 1; or return 1
    contains -- "$argv[1]" 0 130
end
function reject_orphan_leaf
    set -l leaf "$argv[1]"
    set -l expected_start "$argv[2]"
    set -l expected_exe "$argv[3]"
    set -l expected_uid "$argv[4]"
    sudo kill -0 "$leaf" 2>/dev/null; or return 0
    set -l current_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
    set -l current_exe (sudo stat -Lc '%d:%i' "/proc/$leaf/exe" 2>/dev/null)
    set -l current_uid (sudo stat -Lc '%u' "/proc/$leaf" 2>/dev/null)
    if test "$current_start" != "$expected_start"; or test "$current_exe" != "$expected_exe"; or test "$current_uid" != "$expected_uid"
        return 2
    end
    sudo kill -KILL "$leaf"; or return 2
    for attempt in (seq 1 30)
        sudo kill -0 "$leaf" 2>/dev/null; or break
        sleep 0.1
    end
    sudo kill -0 "$leaf" 2>/dev/null; and return 2
    return 1
end
function profile_processes_gone
    set -l wrapper "$argv[1]"
    set -l wrapper_start "$argv[2]"
    set -l leaf "$argv[3]"
    set -l leaf_start "$argv[4]"
    set -l leaf_exe "$argv[5]"
    set -l leaf_uid "$argv[6]"
    if not string match -rq '^[1-9][0-9]*$' -- "$wrapper_start"; or not string match -rq '^[1-9][0-9]*$' -- "$leaf"; or not string match -rq '^[1-9][0-9]*$' -- "$leaf_start"; or test -z "$leaf_exe"; or test -z "$leaf_uid"
        return 1
    end
    if kill -0 "$wrapper" 2>/dev/null
        set -l current_start (sudo awk '{print $22}' "/proc/$wrapper/stat" 2>/dev/null)
        pid_start_proves_gone 1 "$wrapper_start" "$current_start"; or return 1
    end
    if sudo kill -0 "$leaf" 2>/dev/null
        set -l current_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
        pid_start_proves_gone 1 "$leaf_start" "$current_start"; or return 1
    end
    return 0
end
function pid_start_proves_gone
    if test "$argv[1]" = 0
        return 0
    end
    string match -rq '^[1-9][0-9]*$' -- "$argv[2]"; or return 1
    string match -rq '^[1-9][0-9]*$' -- "$argv[3]"; or return 1
    test "$argv[2]" != "$argv[3]"
end
function finish_profile_job
    set -l pid "$argv[1]"
    set -l expected_start "$argv[2]"
    set -l result_path "$argv[3]"
    set -l leaf "$argv[4]"
    set -l leaf_start "$argv[5]"
    set -l leaf_parent "$argv[6]"
    set -l leaf_parent_start "$argv[7]"
    set -l expected_leaf_exe "$argv[8]"
    set -l expected_leaf_uid "$argv[9]"
    set -l signal "$argv[10]"
    if test -f "$result_path"; or not kill -0 "$pid" 2>/dev/null
        wait "$pid" >/dev/null 2>&1
        reject_orphan_leaf "$leaf" "$leaf_start" "$expected_leaf_exe" "$expected_leaf_uid"
        test $status -eq 0; or return 1
        profile_result_ok "$result_path"; or return 1
        return 0
    end
    set -l parent (sudo awk '/^PPid:/ {print $2}' "/proc/$pid/status" 2>/dev/null)
    set -l start (sudo awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
    if test "$parent" != "$fish_pid"; or test "$start" != "$expected_start"
        return 1
    end
    if sudo kill -0 "$leaf" 2>/dev/null
        set -l current_leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$leaf/status" 2>/dev/null)
        set -l current_leaf_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
        set -l current_parent_start (sudo awk '{print $22}' "/proc/$leaf_parent/stat" 2>/dev/null)
        if test "$current_leaf_parent" != "$leaf_parent"; or test "$current_leaf_start" != "$leaf_start"; or test "$current_parent_start" != "$leaf_parent_start"
            return 1
        end
        if test "$leaf_parent" != "$pid"
            set -l intermediate_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$leaf_parent/status" 2>/dev/null)
            test "$intermediate_parent" = "$pid"; or return 1
        end
        if test -n "$signal"
            sudo kill -s "$signal" "$leaf"; or return 1
            if test "$signal" = INT
                set -g recorder_requested_sigint 1
            end
        end
    end
    bounded_wait "$pid" 50
    if test "$bounded_alive" -eq 1
        if sudo kill -0 "$leaf" 2>/dev/null
            set -l current_leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$leaf/status" 2>/dev/null)
            set -l current_leaf_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
            set -l current_leaf_exe (sudo stat -Lc '%d:%i' "/proc/$leaf/exe" 2>/dev/null)
            set -l current_leaf_uid (sudo stat -Lc '%u' "/proc/$leaf" 2>/dev/null)
            test "$current_leaf_parent" = "$leaf_parent"; and test "$current_leaf_start" = "$leaf_start"; and test "$current_leaf_exe" = "$expected_leaf_exe"; and test "$current_leaf_uid" = "$expected_leaf_uid"; or return 1
            sudo kill -KILL "$leaf"; or return 1
            for attempt in (seq 1 30)
                sudo kill -0 "$leaf" 2>/dev/null; or break
                sleep 0.1
            end
            sudo kill -0 "$leaf" 2>/dev/null; and return 1
        end
        if test "$leaf_parent" != "$pid"; and sudo kill -0 "$leaf_parent" 2>/dev/null
            set -l current_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$leaf_parent/status" 2>/dev/null)
            set -l current_start (sudo awk '{print $22}' "/proc/$leaf_parent/stat" 2>/dev/null)
            if test "$current_parent" = "$pid"; and test "$current_start" = "$leaf_parent_start"
                sudo kill -KILL "$leaf_parent"; or return 1
            else
                return 1
            end
        end
        set -l current_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$pid/status" 2>/dev/null)
        set -l current_start (sudo awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
        test "$current_parent" = "$fish_pid"; and test "$current_start" = "$expected_start"; or return 1
        sudo pkill -KILL -P "$pid" >/dev/null 2>&1
        sudo kill -KILL "$pid" >/dev/null 2>&1
        bounded_wait "$pid" 30
        test "$bounded_alive" -eq 0; or return 1
        return 1
    end
    reject_orphan_leaf "$leaf" "$leaf_start" "$expected_leaf_exe" "$expected_leaf_uid"
    test $status -eq 0; or return 1
    profile_result_ok "$result_path"
end
function find_leaf
    set -l root "$argv[1]"
    set -l expected_exe "$argv[2]"
    for wrapper_child in (pgrep -P "$root")
        for candidate in "$wrapper_child" (pgrep -P "$wrapper_child")
            set -l candidate_exe (sudo stat -Lc '%d:%i' "/proc/$candidate/exe" 2>/dev/null)
            if test "$candidate_exe" = "$expected_exe"
                set -l actual_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$candidate/status" 2>/dev/null)
                string match -rq '^[1-9][0-9]*$' -- "$actual_parent"; or continue
                echo "$candidate"
                echo "$actual_parent"
                return 0
            end
        end
    end
    return 1
end
function stop_owned_capture
    if set -q recorder
        set -l ok 0
        finish_profile_job "$recorder" "$recorder_start" "$raw/recorder.result" "$perf_pid" "$perf_start" "$perf_parent" "$perf_parent_start" "$perf_exe" "$perf_uid" INT; or set ok 1
        set recorder_exit (cat "$raw/recorder.status" 2>/dev/null); or set recorder_exit -1
        set -l processes_gone 0
        profile_processes_gone "$recorder" "$recorder_start" "$perf_pid" "$perf_start" "$perf_exe" "$perf_uid"; and set processes_gone 1
        if test "$processes_gone" -eq 1
            set -e recorder
        else
            set -g child_shutdown_failed 1
        end
        if test "$ok" -ne 0; set -g child_shutdown_failed 1; end
    end
    if set -q sampler
        touch "$raw/sampler.stop"; or set -g child_shutdown_failed 1
        set -l ok 0
        finish_profile_job "$sampler" "$sampler_start" "$raw/sampler.result" "$sampler_leaf" "$sampler_leaf_start" "$sampler_parent" "$sampler_parent_start" "$sampler_exe" "$sampler_uid" ""; or set ok 1
        set sampler_exit (cat "$raw/sampler.status" 2>/dev/null); or set sampler_exit -1
        set -l processes_gone 0
        profile_processes_gone "$sampler" "$sampler_start" "$sampler_leaf" "$sampler_leaf_start" "$sampler_exe" "$sampler_uid"; and set processes_gone 1
        if test "$processes_gone" -eq 1
            set -e sampler
        else
            set -g child_shutdown_failed 1
        end
        if test "$ok" -ne 0; set -g child_shutdown_failed 1; end
    end
    if not set -q recorder; and not set -q sampler; and test "$recorder_launched" -eq 1; and test "$sampler_launched" -eq 1
        set -g capture_children_stopped 1
    end
    test "$child_shutdown_failed" -eq 0
end
function acknowledge_cleanup
    set -l done_file "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE"
    set -l stopped_file "$MEDTRACKER_DIAGNOSTIC_COLLECTOR_STOPPED_FILE"
    test -f "$done_file"; or return 1
    set -l run_marker (cat "$done_file"); or return 1
    string match -rq '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' -- "$run_marker"; or return 1
    set -l expected "stopped:$run_marker"
    test "$capture_children_stopped" -eq 1; or return 1
    if test -e "$stopped_file"
        test (cat "$stopped_file") = "$expected"; or return 1
        return 0
    end
    printf '%s\n' "$expected" >"$stopped_file.partial"; or return 1
    mv "$stopped_file.partial" "$stopped_file"
end
function completion_state
    if test -f "$argv[1]"
        set -l marker (cat "$argv[1]")
        if string match -rq '^[0-9]{19}$' -- "$marker"
            echo timeout
            return 0
        end
    end
    if test -f "$argv[2]"
        set -l run_marker (cat "$argv[2]")
        if string match -rq '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' -- "$run_marker"
            echo no_targeted_timeout
        else
            echo invalid_cleanup_handoff
        end
        return 0
    end
    echo waiting
end
function handoff_state
    if not test -f "$argv[1]"
        echo waiting
        return 0
    end
    set -l run_marker (cat "$argv[1]")
    if string match -rq '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' -- "$run_marker"
        echo ready
    else
        echo invalid
    end
end
function await_cleanup_handoff
    set -q MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE MEDTRACKER_DIAGNOSTIC_COLLECTOR_STOPPED_FILE; or return 1
    test "$capture_children_stopped" -eq 1; or return 1
    set -l state (handoff_state "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
    if test "$state" = ready
        acknowledge_cleanup
        return $status
    end
    test "$state" = waiting; or return 1
    set -q await_deadline_ms; or return 1
    set -l remaining (node -e 'const d=Number(process.argv[1])-Date.now();process.stdout.write(String(Math.max(0,Math.ceil(d/250))))' "$await_deadline_ms"); or return 1
    for attempt in (seq 1 "$remaining")
        set state (handoff_state "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
        if test "$state" = ready
            acknowledge_cleanup
            return $status
        end
        test "$state" = waiting; or return 1
        sleep 0.25; or return 1
    end
    return 1
end
if test "$argv[1]" = --lifecycle-selftest
    set -g selftest_mode 1
    pid_start_proves_gone 0 10 invalid; or exit 1
    pid_start_proves_gone 1 10 10; and exit 1
    pid_start_proves_gone 1 10 11; or exit 1
    pid_start_proves_gone 1 10 unreadable; and exit 1
    recorder_status_allowed 130 1 1; or exit 1
    recorder_status_allowed 130 0 1; and exit 1
    recorder_status_allowed 130 1 0; and exit 1
    recorder_status_allowed 143 1 1; and exit 1
    recorder_status_allowed 0 0 1; and exit 1
    recorder_status_allowed 0 1 1; or exit 1
    set -l test_dir (mktemp -d); or exit 1
    set -g out "$test_dir"
    set -g raw "$test_dir/private"
    mkdir "$raw"; or exit 1
    set -gx MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE "$test_dir/done"
    set -gx MEDTRACKER_DIAGNOSTIC_COLLECTOR_STOPPED_FILE "$test_dir/stopped"
    set -gx run_marker 6a70af3a-9852-4c2a-8d2e-b2e46b931f94
    child_result "$test_dir/missing.result"; and exit 1
    profile_result_ok "$test_dir/missing.result"; and exit 1
    for code in 0 7
        node -e "$child_wrapper" "$test_dir/$code.result" fish -c "exit $code" &
        set -l child $last_pid
        bounded_wait "$child" 50; or exit 1
        test (cat "$test_dir/$code.result") = "$code"; or exit 1
        if test "$code" -eq 0
            child_result "$test_dir/$code.result"; or exit 1
            profile_result_ok "$test_dir/$code.result"; or exit 1
            test (cat "$test_dir/$code.status") = 0; or exit 1
        else
            child_result "$test_dir/$code.result"; and exit 1
            profile_result_ok "$test_dir/$code.result"; and exit 1
            test (cat "$test_dir/$code.status") = "$code"; or exit 1
        end
    end
    node -e "$child_wrapper" "$test_dir/signal.result" fish -c 'kill -TERM $fish_pid' &
    set -l signalled_child $last_pid
    bounded_wait "$signalled_child" 50; or exit 1
    test (cat "$test_dir/signal.result") = 143; or exit 1
    child_result "$test_dir/signal.result"; and exit 1
    set -l node_path (command -s node); or exit 1
    node -e "$child_wrapper" "$test_dir/interrupt.result" "$node_path" -e 'require("fs").writeFileSync(process.argv[1], String(process.pid)); setInterval(() => {}, 1000)' "$test_dir/interrupt.pid" &
    set -l interrupt_wrapper $last_pid
    for attempt in (seq 1 50)
        test -s "$test_dir/interrupt.pid"; and break
        sleep 0.1
    end
    set -l interrupt_child (cat "$test_dir/interrupt.pid"); or exit 1
    kill -INT "$interrupt_child"; or exit 1
    bounded_wait "$interrupt_wrapper" 50; or exit 1
    test (cat "$test_dir/interrupt.result") = 130; or exit 1
    child_result "$test_dir/interrupt.result"; and exit 1
    test (completion_state "$test_dir/timeout" "$test_dir/done") = waiting; or exit 1
    printf '%s\n' "$run_marker" >"$test_dir/done"
    test (completion_state "$test_dir/timeout" "$test_dir/done") = no_targeted_timeout; or exit 1
    printf 'bad\n' >"$test_dir/done"
    test (completion_state "$test_dir/timeout" "$test_dir/done") = invalid_cleanup_handoff; or exit 1
    printf '1234567890123456789\n' >"$test_dir/timeout"
    test (completion_state "$test_dir/timeout" "$test_dir/done") = timeout; or exit 1
    test (handoff_state "$test_dir/done") = invalid; or exit 1
    rm "$test_dir/timeout"
    rm "$test_dir/done"
    test (handoff_state "$test_dir/done") = waiting; or exit 1
    printf '0\n' >"$raw/recorder.result"
    printf '7\n' >"$raw/sampler.result"
    set -g completed_timeout_capture 1
    set -g capture_children_stopped 0
    test "$completed_timeout_capture" -eq 1; or exit 1
    test (handoff_state "$test_dir/done") = waiting; or exit 1
    printf '1234567890123456789\n' >"$test_dir/timeout"
    printf '%s\n' "$run_marker" >"$test_dir/done"
    test (completion_state "$test_dir/timeout" "$test_dir/done") = timeout; or exit 1
    test (handoff_state "$test_dir/done") = ready; or exit 1
    acknowledge_cleanup; and exit 1
    test ! -e "$test_dir/stopped"; or exit 1
    set -g capture_children_stopped 1
    acknowledge_cleanup; or exit 1
    test (cat "$test_dir/stopped") = "stopped:$run_marker"; or exit 1
    rm "$test_dir/done" "$test_dir/stopped"
    set -g await_deadline_ms (node -e 'process.stdout.write(String(Date.now()+5000))'); or exit 1
    set -g reason selftest_report_failure
    fish --no-config -c 'sleep 0.2; printf "%s\n" "$argv[1]" >"$argv[2].partial"; and mv "$argv[2].partial" "$argv[2]"' "$run_marker" "$test_dir/done" &
    set -l handoff_writer $last_pid
    await_cleanup_handoff; or exit 1
    wait "$handoff_writer"; or exit 1
    test "$reason" = selftest_report_failure; or exit 1
    test (cat "$test_dir/stopped") = "stopped:$run_marker"; or exit 1
    rm -rf "$test_dir"
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
function cleanup --on-event fish_exit
    test "$selftest_mode" -eq 0; or return
    if test -f "$raw/paused"
        docker unpause "$service" >/dev/null 2>&1
        rm -f "$raw/paused"
    end
    if set -q recorder; or set -q sampler
        for attempt in (seq 1 3)
            stop_owned_capture
            if not set -q recorder; and not set -q sampler
                break
            end
            sleep 0.1
        end
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
    if set -q MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE MEDTRACKER_DIAGNOSTIC_COLLECTOR_STOPPED_FILE; and test "$capture_children_stopped" -eq 1
        await_cleanup_handoff
    end
    printf 'run_id=%s source_sha=%s phase=%s reason=%s initial_count=%s a_count=%s b_count=%s snapshot_success=%s recorder_exit=%s sampler_exit=%s child_shutdown_failed=%s duration_ms=%s report_exit=%s %s %s\n' "$GITHUB_RUN_ID" "$GITHUB_SHA" "$phase" "$reason" "$initial_count" "$a_count" "$b_count" "$snapshot_success" "$recorder_exit" "$sampler_exit" "$child_shutdown_failed" "$duration_ms" "$report_exit" "$report_info" "$summary" | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/status.age.partial" 2>/dev/null
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
set -g perf_path (command -s perf); or exit 1
set -g perf_exe (sudo stat -Lc '%d:%i' "$perf_path"); or exit 1
rm -f "$raw/recorder.result" "$raw/recorder.status"
node -e "$child_wrapper" "$raw/recorder.result" sudo "$perf_path" record -e cpu-clock -p "$pidlist" -F 49 --strict-freq --max-size 256M --clockid mono --timestamp-boundary --no-buildid-cache -o "$raw/perf.data" >"$raw/perf.log" 2>&1 &
set -g recorder $last_pid
set -g recorder_launched 1
set -g recorder_start (sudo awk '{print $22}' "/proc/$recorder/stat"); or exit 1
set -l perf_pid
set -l perf_parent
for attempt in (seq 1 80)
    set -l leaf_tree (find_leaf "$recorder" "$perf_exe")
    if test (count $leaf_tree) -eq 2
        set perf_pid "$leaf_tree[1]"
        set perf_parent "$leaf_tree[2]"
    end
    if set -q perf_pid
        set -l count_events (sudo find "/proc/$perf_pid/fd" -type l -exec readlink '{}' ';' 2>/dev/null | string match -r 'anon_inode:\[perf_event\]' | count)
        if test "$count_events" -ge (count $pids); break; end
    end
    kill -0 "$recorder" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
set -g perf_parent_start (sudo awk '{print $22}' "/proc/$perf_parent/stat"); or exit 1
set -g perf_start (sudo awk '{print $22}' "/proc/$perf_pid/stat"); or exit 1
set -g perf_uid (sudo stat -Lc '%u' "/proc/$perf_pid"); or exit 1
process_snapshot >"$raw/after"; or exit 1
cmp -s "$raw/before" "$raw/after"; or exit 1
docker unpause "$service" >/dev/null; or exit 1
rm "$raw/paused"; or exit 1
set reason sampler_start
set -l node_path (command -s node); or exit 1
set -g sampler_exe (sudo stat -Lc '%d:%i' "$node_path"); or exit 1
rm -f "$raw/sampler.result" "$raw/sampler.status"
node -e "$child_wrapper" "$raw/sampler.result" sudo "$node_path" scripts/diagnostics/pg-capture-sampler.js "$raw" "$cgdir" "$postmaster" "$service" >"$raw/sampler.log" 2>&1 &
set -g sampler $last_pid
set -g sampler_launched 1
set -l sampler_parent
set -l sampler_leaf
for attempt in (seq 1 80)
    test -f "$raw/sampler.ready"; and break
    set -l leaf_tree (find_leaf "$sampler" "$sampler_exe")
    if test (count $leaf_tree) -eq 2
        set sampler_leaf "$leaf_tree[1]"
        set sampler_parent "$leaf_tree[2]"
    end
    kill -0 "$sampler" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
for attempt in (seq 1 80)
    if test -n "$sampler_leaf"
        break
    end
    set -l leaf_tree (find_leaf "$sampler" "$sampler_exe")
    if test (count $leaf_tree) -eq 2
        set sampler_leaf "$leaf_tree[1]"
        set sampler_parent "$leaf_tree[2]"
    end
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
set -g sampler_parent "$sampler_parent"
set -g sampler_leaf "$sampler_leaf"
set -g sampler_parent_start (sudo awk '{print $22}' "/proc/$sampler_parent/stat"); or exit 1
set -g sampler_leaf_start (sudo awk '{print $22}' "/proc/$sampler_leaf/stat"); or exit 1
set -g sampler_uid (sudo stat -Lc '%u' "/proc/$sampler_leaf"); or exit 1
set -g sampler_start (sudo awk '{print $22}' "/proc/$sampler/stat"); or exit 1
touch "$out/collector.ready"; or exit 1
set phase await_timeout
set reason wait_marker
set -g await_deadline_ms (node -e 'process.stdout.write(String(Date.now()+1800000))'); or exit 1
set -l found 0
set -l tests_done 0
for attempt in (seq 1 7200)
    set -l state (completion_state "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE" "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
    switch "$state"
        case timeout
            set -g marker (cat "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE")
            set found 1
            break
        case no_targeted_timeout invalid_cleanup_handoff
            set tests_done 1
            break
    end
    kill -0 "$recorder" 2>/dev/null; and kill -0 "$sampler" 2>/dev/null; or exit 1
    sleep 0.25
end
if test "$found" -eq 0
    test "$tests_done" -eq 1; or exit 1
    set phase complete
    set reason (completion_state "$MEDTRACKER_DIAGNOSTIC_TIMEOUT_FILE" "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
    stop_owned_capture; or exit 1
    if test "$reason" = no_targeted_timeout
        acknowledge_cleanup; or exit 1
        printf 'no_targeted_timeout\n' >"$out/no-targeted-timeout"; or exit 1
    end
    if test "$reason" = no_targeted_timeout
        exit 3
    end
    exit 1
end
set phase after_timeout
set reason wait_recording
for attempt in (seq 1 240)
    set -l now_ns (node -e 'process.stdout.write(String(BigInt(Date.now())*1000000n))'); or exit 1
    set -l elapsed (node -e 'const m=BigInt(process.argv[1]),n=BigInt(process.argv[2]);process.stdout.write(String(n-m))' "$marker" "$now_ns"); or exit 1
    set -l state (handoff_state "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
    if test "$elapsed" -ge 3000000000; and test "$state" = ready
        set tests_done 1
        break
    end
    test "$elapsed" -lt 60000000000; or break
    sleep 0.25; or exit 1
end
set -l final_state (handoff_state "$MEDTRACKER_DIAGNOSTIC_CLEANUP_DONE_FILE")
if test "$final_state" = ready
    set tests_done 1
end
set reason stop_recording
stop_owned_capture; or exit 1
if test "$tests_done" -eq 1
    acknowledge_cleanup; or exit 1
end
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
if test "$tests_done" -eq 0
    set phase await_cleanup
    set reason wait_cleanup_handoff
    await_cleanup_handoff; or exit 1
    set tests_done 1
end
set phase complete
