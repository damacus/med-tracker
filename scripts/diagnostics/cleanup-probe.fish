#!/usr/bin/env fish
# Temporary owned-service diagnostic. Only fixed stages and safe wait metadata leave this host.
umask 077
set -g child_wrapper 'set -l result $argv[1]; $argv[2..]; set -l code $status; printf "%s\n" $code > "$result.tmp"; and mv "$result.tmp" "$result"'
function bounded_wait
    set -g bounded_alive 0
    for attempt in (seq 1 "$argv[2]")
        kill -0 "$argv[1]" 2>/dev/null; or begin; wait "$argv[1]" >/dev/null 2>&1; return 0; end
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
function record_result
    set -l value (cat "$argv[1]" 2>/dev/null)
    if string match -rq '^(0|[1-9][0-9]{0,2})$' -- "$value"; and test "$value" -le 255
        event "$argv[2].exit=$value"
    else
        event "$argv[2].status_missing_or_invalid"
    end
end
if test "$argv[1]" = --shutdown-selftest
    echo shutdown_selftest_start
    set -l tmp (mktemp -d); or exit 1
    child_result "$tmp/missing"; and exit 1
    for code in 0 7
        fish --no-config -c "$child_wrapper" "$tmp/$code" fish -c "exit $code" &
        set -l pid $last_pid
        bounded_wait "$pid" 50; or exit 1
        test (cat "$tmp/$code") = "$code"; or exit 1
        if test "$code" -eq 0; child_result "$tmp/$code"; or exit 1
        else; child_result "$tmp/$code"; and exit 1; end
    end
    fish --no-config -c "$child_wrapper" "$tmp/signal" fish -c 'kill -TERM $fish_pid' &
    set -l pid $last_pid
    bounded_wait "$pid" 50; or exit 1
    test (cat "$tmp/signal") = 143; or exit 1
    child_result "$tmp/signal"; and exit 1
    rm -rf "$tmp"
    echo shutdown_selftest_passed
    exit 0
end
set -g nested_selftest 0
if test "$argv[1]" = --nested-selftest
    set -g nested_selftest 1
    set -g out (mktemp -d); or exit 1
    set -g raw "$out"
else
    test "$GITHUB_ACTIONS" = true; or exit 1
    string match -rq '^[0-9a-f]{64}$' "$PG_DIAGNOSTIC_CONTAINER"; or exit 1
    test (docker inspect --format '{{.Config.Image}}' "$PG_DIAGNOSTIC_CONTAINER") = postgres:18-alpine; or exit 1
    set -g out "$RUNNER_TEMP/cleanup-probe"
    set -g raw "$out/private"
    mkdir -m 700 "$out" "$raw"; or exit 1
    set -g service "$PG_DIAGNOSTIC_CONTAINER"
end
function event
    printf '%s %s\n' (date -u +%s%N) "$argv[1]" >>"$raw/events"
end
function cleanup --on-event fish_exit
    if test "$nested_selftest" -eq 1
        if set -q sampler sampler_start
            finish_profile_job "$sampler" "$sampler_start" sampler 2
        end
        if test -f "$raw/events"; cat "$raw/events"; end
        rm -rf -- "$out"
        return
    end
    if test -f "$raw/paused"
        docker unpause "$service" >/dev/null 2>&1
        rm -f "$raw/paused"
    end
    if set -q recorder
        set -q recorder_start; or set -g recorder_start invalid
        set -l parent (sudo awk '/^PPid:/ {print $2}' "/proc/$recorder/status" 2>/dev/null)
        set -l start (sudo awk '{print $22}' "/proc/$recorder/stat" 2>/dev/null)
        if set -q recorder_leaf recorder_leaf_start recorder_leaf_parent recorder_leaf_parent_start; and test "$parent" = "$fish_pid"; and test "$start" = "$recorder_start"
            set -l leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$recorder_leaf/status" 2>/dev/null)
            set -l leaf_start (sudo awk '{print $22}' "/proc/$recorder_leaf/stat" 2>/dev/null)
            set -l owner_start (sudo awk '{print $22}' "/proc/$recorder_leaf_parent/stat" 2>/dev/null)
            if test "$leaf_parent" = "$recorder_leaf_parent"; and test "$leaf_start" = "$recorder_leaf_start"; and test "$owner_start" = "$recorder_leaf_parent_start"
                sudo kill -INT "$recorder_leaf" >/dev/null 2>&1
            end
        end
        finish_profile_job "$recorder" "$recorder_start" recorder
        if test "$profile_ended" -eq 1; set -e recorder; end
    end
    if set -q sampler
        set -q sampler_start; or set -g sampler_start invalid
        touch "$raw/sampler.stop"
        finish_profile_job "$sampler" "$sampler_start" sampler
        if test "$profile_ended" -eq 1; set -e sampler; end
    end
    if set -q cleanup_job
        if not kill -0 "$cleanup_job" 2>/dev/null
            wait "$cleanup_job" >/dev/null 2>&1
        else if not terminate_owned_cleanup
            event cleanup_termination_unverified
        end
    end
    if set -q observer
        set -q observer_start; or set -g observer_start invalid
        finish_profile_job "$observer" "$observer_start" observer 150
        if test "$profile_ended" -eq 1; set -e observer; end
    end
    if test -f "$raw/events"
        begin
            cat "$raw/events"
            if test -f "$raw/baseline-stages"; printf 'baseline_stages\n'; cat "$raw/baseline-stages"; end
            if test -f "$raw/instrumented-stages"; printf 'instrumented_stages\n'; cat "$raw/instrumented-stages"; end
            if test -f "$raw/waits"; cat "$raw/waits"; end
        end | age -r "$MEDTRACKER_PGSS_RECIPIENT" -o "$out/safe-evidence.age.partial" 2>/dev/null
        set -l status_list $pipestatus
        if test $status_list[1] -eq 0; and test $status_list[2] -eq 0; and test -s "$out/safe-evidence.age.partial"
            mv "$out/safe-evidence.age.partial" "$out/safe-evidence.age"
        end
    end
    find "$out" -maxdepth 1 -name '*.partial' -delete
    sudo rm -rf -- "$raw"
end
function finish_profile_job
    set -g profile_ended 0
    set -l pid "$argv[1]"
    set -l expected_start "$argv[2]"
    set -l label "$argv[3]"
    set -l attempts 50
    if test (count $argv) -ge 4; set attempts "$argv[4]"; end
    bounded_wait "$pid" "$attempts"
    if test "$bounded_alive" -eq 0
        if contains -- "$label" recorder sampler
            set -l leaf_name "$label"_leaf
            set -l start_name "$label"_leaf_start
            set -l exe_name "$label"_leaf_exe
            set -l uid_name "$label"_leaf_uid
            if not set -q $leaf_name $start_name $exe_name $uid_name
                event "$label.leaf_unverified_incomplete"
                return 1
            end
            set -l leaf $$leaf_name
            if sudo kill -0 "$leaf" 2>/dev/null
                set -l actual_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
                set -l actual_exe (sudo stat -Lc '%d:%i' "/proc/$leaf/exe" 2>/dev/null)
                set -l actual_uid (sudo stat -Lc '%u' "/proc/$leaf" 2>/dev/null)
                if test "$actual_start" != "$$start_name"; or test "$actual_exe" != "$$exe_name"; or test "$actual_uid" != "$$uid_name"
                    event "$label.orphan_identity_changed_incomplete"
                    return 1
                end
                event "$label.orphan_forced_incomplete"
                sudo kill -KILL "$leaf" 2>/dev/null; or return 1
                for attempt in (seq 1 30)
                    sudo kill -0 "$leaf" 2>/dev/null; or break
                    sleep 0.1
                end
                if sudo kill -0 "$leaf" 2>/dev/null
                    event "$label.orphan_survived_incomplete"
                    return 1
                end
                set -g profile_ended 1
                return 1
            end
        end
        set -g profile_ended 1
        record_result "$raw/$label.result" "$label"
        if child_result "$raw/$label.result"
            event "$label.exit=0"
            return 0
        end
        event "$label.failed_or_missing_status"
        return 1
    end
    set -l parent (sudo awk '/^PPid:/ {print $2}' "/proc/$pid/status" 2>/dev/null)
    set -l start (sudo awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
    if test "$parent" != "$fish_pid"; or test "$start" != "$expected_start"
        event "$label.termination_unverified"
        return 1
    end
    event "$label.forced_incomplete"
    if contains -- "$label" recorder sampler
        set -l leaf_name "$label"_leaf
        set -l parent_name "$label"_leaf_parent
        set -l start_name "$label"_leaf_start
        set -l parent_start_name "$label"_leaf_parent_start
        if not set -q $leaf_name $parent_name $start_name $parent_start_name
            event "$label.leaf_unverified_incomplete"
            return 1
        end
        set -l leaf $$leaf_name
        if sudo kill -0 "$leaf" 2>/dev/null
            set -l leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$leaf/status" 2>/dev/null)
            set -l leaf_start (sudo awk '{print $22}' "/proc/$leaf/stat" 2>/dev/null)
            set -l owner_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$$parent_name/status" 2>/dev/null)
            set -l owner_start (sudo awk '{print $22}' "/proc/$$parent_name/stat" 2>/dev/null)
            if test "$leaf_parent" != "$$parent_name"; or test "$leaf_start" != "$$start_name"; or test "$owner_start" != "$$parent_start_name"; or begin; test "$$parent_name" != "$pid"; and test "$owner_parent" != "$pid"; end
                event "$label.leaf_identity_changed_incomplete"
                return 1
            end
            sudo kill -KILL "$leaf" 2>/dev/null; or return 1
            for attempt in (seq 1 30)
                sudo kill -0 "$leaf" 2>/dev/null; or break
                sleep 0.1
            end
            if sudo kill -0 "$leaf" 2>/dev/null
                event "$label.leaf_survived_incomplete"
                return 1
            end
        end
    end
    if kill -0 "$pid" 2>/dev/null
        set -l current_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$pid/status" 2>/dev/null)
        set -l current_start (sudo awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
        if test "$current_parent" != "$fish_pid"; or test "$current_start" != "$expected_start"
            event "$label.wrapper_identity_changed_incomplete"
            return 1
        end
        sudo pkill -KILL -P "$pid" >/dev/null 2>&1
        sudo kill -KILL "$pid" >/dev/null 2>&1
    end
    bounded_wait "$pid" 30
    if test "$bounded_alive" -eq 1
        event "$label.unreaped_incomplete"
    else
        set -g profile_ended 1
        event "$label.terminated_incomplete"
    end
    return 1
end
function stop_owned_cleanup
    set -l signal TERM
    if test (count $argv) -gt 0; set signal "$argv[1]"; end
    contains -- "$signal" TERM KILL; or return 1
    set -l parent (awk '/^PPid:/ {print $2}' "/proc/$cleanup_job/status" 2>/dev/null)
    set -l start (awk '{print $22}' "/proc/$cleanup_job/stat" 2>/dev/null)
    if set -q cleanup_start; and test "$parent" = "$fish_pid"; and test "$start" = "$cleanup_start"
        event cleanup_termination_requested
        pkill -$signal -P "$cleanup_job" 2>/dev/null; and return 0
    end
    return 1
end
function terminate_owned_cleanup
    stop_owned_cleanup TERM; or return 1
    for attempt in (seq 1 3)
        kill -0 "$cleanup_job" 2>/dev/null; or break
        sleep 1
    end
    if kill -0 "$cleanup_job" 2>/dev/null
        event cleanup_force_kill_requested
        stop_owned_cleanup KILL; or return 1
    end
    bounded_wait "$cleanup_job" 30
    if test "$bounded_alive" -eq 1
        event cleanup_unreaped_incomplete
        return 1
    end
    event cleanup_terminated_incomplete
    return 0
end
function interrupted --on-signal SIGINT
    exit 1
end
function terminated --on-signal SIGTERM
    exit 1
end
if test "$nested_selftest" -eq 1
    event selftest_start
    test (uname -s) = Linux; or exit 1
    set -l node_path (command -s node); or exit 1
    set -l node_exe (sudo stat -Lc '%d:%i' "$node_path"); or exit 1
    for mode in nested orphan
    event "selftest_$mode.start"
    rm -f "$raw/sampler.result"
    set -e sampler_leaf
    fish --no-config -c "$child_wrapper" "$raw/sampler.result" sudo "$node_path" -e 'process.on("SIGTERM", () => {}); setInterval(() => {}, 1000)' >/dev/null 2>&1 &
    set -g sampler $last_pid
    set -g sampler_start (sudo awk '{print $22}' "/proc/$sampler/stat"); or exit 1
    for attempt in (seq 1 80)
        for parent in (pgrep -P "$sampler")
            for p in "$parent" (pgrep -P "$parent")
                set -l candidate_exe (sudo stat -Lc '%d:%i' "/proc/$p/exe" 2>/dev/null)
                if test "$candidate_exe" = "$node_exe"
                    set -g sampler_leaf "$p"
                    set -g sampler_leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$p/status")
                end
            end
        end
        set -q sampler_leaf; and break
        sleep 0.1
    end
    event "selftest_$mode.discovery_done"
    set -q sampler_leaf; or exit 1
    set -g sampler_leaf_start (sudo awk '{print $22}' "/proc/$sampler_leaf/stat"); or exit 1
    set -g sampler_leaf_exe (sudo stat -Lc '%d:%i' "/proc/$sampler_leaf/exe"); or exit 1
    set -g sampler_leaf_uid (sudo stat -Lc '%u' "/proc/$sampler_leaf"); or exit 1
    set -g sampler_leaf_parent_start (sudo awk '{print $22}' "/proc/$sampler_leaf_parent/stat"); or exit 1
    event "selftest_$mode.identity_recorded"
    sleep 1
    sudo kill -TERM "$sampler_leaf"; or exit 1
    sleep 0.2
    sudo kill -0 "$sampler_leaf" 2>/dev/null; or exit 1
    event "selftest_$mode.term_ignored"
    if test "$mode" = orphan
        test (sudo awk '{print $22}' "/proc/$sampler_leaf_parent/stat") = "$sampler_leaf_parent_start"; or exit 1
        test (sudo awk '{print $22}' "/proc/$sampler/stat") = "$sampler_start"; or exit 1
        event selftest_orphan.parent_kill_start
        sudo kill -KILL "$sampler_leaf_parent"; or exit 1
        event selftest_orphan.wrapper_wait_start
        bounded_wait "$sampler" 30; or exit 1
        event selftest_orphan.wrapper_wait_complete
        sudo kill -0 "$sampler_leaf" 2>/dev/null; or exit 1
        event selftest_orphan.parents_gone_leaf_alive
    end
    event "selftest_$mode.finish_start"
    finish_profile_job "$sampler" "$sampler_start" sampler 2
    set -l forced_result $status
    event "selftest_$mode.finish_result=$forced_result"
    test "$forced_result" -ne 0; and test "$profile_ended" -eq 1; or exit 1
    sudo kill -0 "$sampler_leaf" 2>/dev/null; and exit 1
    child_result "$raw/sampler.result"; and exit 1
    event "selftest_$mode.passed"
    set -e sampler
    end
    exit 0
end
set -l target_dir (cargo metadata --locked --no-deps --format-version 1 | jq -er .target_directory); or exit 1
set -g fixture "$target_dir/debug/examples/api-fixtures"
set -g observer_binary "$target_dir/debug/examples/cleanup-waits"
test -x "$fixture"; and test -x "$observer_binary"; or exit 1
set -gx MEDTRACKER_OWNED_DATABASE 1
set -gx NEXTEST_ENV /dev/null
set -gx MEDTRACKER_TEST_RUN_ID (uuidgen | string lower)
event baseline_prepare_start
timeout 120 "$fixture" prepare >"$raw/baseline-prepare.log" 2>&1
set -l prepare_exit $status
event "baseline_prepare_exit=$prepare_exit"
if test "$prepare_exit" -eq 124; event baseline_prepare_incomplete; exit 1; end
if test "$prepare_exit" -ne 0; event baseline_prepare_failed; exit 1; end
event baseline_prepare_complete
set -gx MEDTRACKER_CLEANUP_STAGE_FILE "$raw/baseline-stages"
fish --no-config -c "$child_wrapper" "$raw/baseline.result" "$fixture" cleanup >"$raw/baseline-cleanup.log" 2>&1 &
set -g cleanup_job $last_pid
set -g cleanup_start (awk '{print $22}' "/proc/$cleanup_job/stat"); or exit 1
event baseline_cleanup_start
for attempt in (seq 1 30)
    kill -0 "$cleanup_job" 2>/dev/null; or break
    sleep 1
end
if kill -0 "$cleanup_job" 2>/dev/null
    event baseline_cleanup_incomplete
    terminate_owned_cleanup; or begin; event cleanup_termination_unverified; set -e cleanup_job; exit 1; end
    set -e cleanup_job
    exit 1
end
wait "$cleanup_job"
record_result "$raw/baseline.result" baseline_cleanup
child_result "$raw/baseline.result"
set -l baseline_exit $status
event "baseline_cleanup_result=$baseline_exit"
set -e cleanup_job
test -s "$raw/baseline-stages"; or begin; event baseline_stage_missing; exit 1; end
if test "$baseline_exit" -ne 0; event baseline_cleanup_failed; exit 1; end
event baseline_cleanup_complete
set -g postmaster (docker inspect --format '{{.State.Pid}}' "$service"); or exit 1
string match -rq '^[1-9][0-9]*$' "$postmaster"; or exit 1
set -l cg (string replace -r '^0::' '' -- (cat "/proc/$postmaster/cgroup")); or exit 1
string match -q "*$service*" "$cg"; or exit 1
set -g cgdir "/sys/fs/cgroup$cg"
test -f "$cgdir/cgroup.procs"; or exit 1
set -g exe (sudo stat -Lc '%d:%i' "/proc/$postmaster/exe"); or exit 1
test "$exe" = (sudo stat -Lc '%d:%i' "/proc/$postmaster/root/usr/local/bin/postgres"); or exit 1
function pg_pids
    for p in (cat "$cgdir/cgroup.procs")
        set -l this_exe (sudo stat -Lc '%d:%i' "/proc/$p/exe" 2>/dev/null); or return 1
        test "$this_exe" = "$exe"; or continue
        set -l ancestor "$p"
        for depth in (seq 1 32)
            if test "$ancestor" = "$postmaster"; echo "$p"; break; end
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
        set -l fields (string split ' ' -- (string replace -r '^.*\) ' '' -- "$statline"))
        set -l ns (sudo awk '/^NSpid:/ {print $NF}' "/proc/$p/status"); or return 1
        set -a rows "$p $fields[2] $fields[20] $ns"
    end
    printf '%s\n' $rows | sort -n
end
touch "$raw/paused"; or exit 1
fish --no-config -c 'sleep 20; if test -f "$argv[1]"; docker unpause "$argv[2]" >/dev/null 2>&1; end' "$raw/paused" "$service" &
docker pause "$service" >/dev/null; or exit 1
process_snapshot >"$raw/before"; or exit 1
set -l pids (awk '{print $1}' "$raw/before")
contains -- "$postmaster" $pids; or exit 1
set -l pidlist (string join ',' $pids)
fish --no-config -c "$child_wrapper" "$raw/recorder.result" sudo perf record -e cpu-clock -p "$pidlist" -F 49 --strict-freq --max-size 256M --clockid mono --timestamp-boundary --no-buildid-cache -o "$raw/perf.data" >"$raw/perf.log" 2>&1 &
set -g recorder $last_pid
set -g recorder_start (sudo awk '{print $22}' "/proc/$recorder/stat"); or exit 1
set -l perf_pid
for attempt in (seq 1 80)
    for wrapper_child in (pgrep -P "$recorder")
        for p in "$wrapper_child" (pgrep -P "$wrapper_child")
            string match -rq '^perf' -- (ps -o comm= -p "$p" | string trim); and set perf_pid "$p"
        end
    end
    if test -n "$perf_pid"
        set -l count_events (sudo find "/proc/$perf_pid/fd" -type l -exec readlink '{}' ';' 2>/dev/null | string match -r 'anon_inode:\[perf_event\]' | count)
        if test "$count_events" -ge (count $pids); break; end
    end
    kill -0 "$recorder" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
set -g recorder_leaf "$perf_pid"
set -g recorder_leaf_start (sudo awk '{print $22}' "/proc/$perf_pid/stat"); or exit 1
set -g recorder_leaf_exe (sudo stat -Lc '%d:%i' "/proc/$perf_pid/exe"); or exit 1
set -g recorder_leaf_uid (sudo stat -Lc '%u' "/proc/$perf_pid"); or exit 1
set -l perf_start "$recorder_leaf_start"
set -g recorder_leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$perf_pid/status"); or exit 1
set -g recorder_leaf_parent_start (sudo awk '{print $22}' "/proc/$recorder_leaf_parent/stat"); or exit 1
process_snapshot >"$raw/after"; or exit 1
cmp -s "$raw/before" "$raw/after"; or exit 1
docker unpause "$service" >/dev/null; or exit 1
rm "$raw/paused"; or exit 1
set -l node_path (command -s node); or exit 1
set -l node_exe (sudo stat -Lc '%d:%i' "$node_path"); or exit 1
fish --no-config -c "$child_wrapper" "$raw/sampler.result" sudo "$node_path" scripts/diagnostics/pg-capture-sampler.js "$raw" "$cgdir" "$postmaster" "$service" >"$raw/sampler.log" 2>&1 &
set -g sampler $last_pid
set -g sampler_start (sudo awk '{print $22}' "/proc/$sampler/stat"); or exit 1
for attempt in (seq 1 80)
    for wrapper_child in (pgrep -P "$sampler")
        for p in "$wrapper_child" (pgrep -P "$wrapper_child")
            set -l candidate_exe (sudo stat -Lc '%d:%i' "/proc/$p/exe" 2>/dev/null)
            if test "$candidate_exe" = "$node_exe"
                set -g sampler_leaf "$p"
            end
        end
    end
    set -q sampler_leaf; and break
    kill -0 "$sampler" 2>/dev/null; or exit 1
    sleep 0.1
end
set -q sampler_leaf; or exit 1
set -g sampler_leaf_start (sudo awk '{print $22}' "/proc/$sampler_leaf/stat"); or exit 1
set -g sampler_leaf_exe (sudo stat -Lc '%d:%i' "/proc/$sampler_leaf/exe"); or exit 1
set -g sampler_leaf_uid (sudo stat -Lc '%u' "/proc/$sampler_leaf"); or exit 1
set -g sampler_leaf_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$sampler_leaf/status"); or exit 1
set -g sampler_leaf_parent_start (sudo awk '{print $22}' "/proc/$sampler_leaf_parent/stat"); or exit 1
for attempt in (seq 1 80)
    test -f "$raw/sampler.ready"; and break
    kill -0 "$sampler" 2>/dev/null; or exit 1
    test "$attempt" -lt 80; or exit 1
    sleep 0.1
end
event profiler_and_sampler_ready
set -e MEDTRACKER_CLEANUP_STAGE_FILE
set -gx MEDTRACKER_TEST_RUN_ID (uuidgen | string lower)
event instrumented_prepare_start
timeout 120 "$fixture" prepare >"$raw/instrumented-prepare.log" 2>&1
set prepare_exit $status
event "instrumented_prepare_exit=$prepare_exit"
if test "$prepare_exit" -eq 124; event instrumented_prepare_incomplete; exit 1; end
if test "$prepare_exit" -ne 0; event instrumented_prepare_failed; exit 1; end
event instrumented_prepare_complete
kill -0 "$recorder" 2>/dev/null; and kill -0 "$sampler" 2>/dev/null; or begin; event profiler_or_sampler_early_exit; exit 1; end
set -gx MEDTRACKER_CLEANUP_STAGE_FILE "$raw/instrumented-stages"
fish --no-config -c "$child_wrapper" "$raw/instrumented.result" "$fixture" cleanup >"$raw/instrumented-cleanup.log" 2>&1 &
set -g cleanup_job $last_pid
set -g cleanup_start (awk '{print $22}' "/proc/$cleanup_job/stat"); or exit 1
event instrumented_cleanup_start
for attempt in (seq 1 5)
    kill -0 "$cleanup_job" 2>/dev/null; or break
    sleep 1
end
if kill -0 "$cleanup_job" 2>/dev/null
    event blocked_at_observation
    fish --no-config -c "$child_wrapper" "$raw/observer.result" "$observer_binary" >"$raw/waits" 2>"$raw/observer.log" &
    set -g observer $last_pid
    set -g observer_start (awk '{print $22}' "/proc/$observer/stat"); or exit 1
    set -l observed 0
    for attempt in (seq 1 30)
        if test -s "$raw/waits"; and jq -e -s 'length > 0' "$raw/waits" >/dev/null 2>&1
            set observed 1
            break
        end
        kill -0 "$observer" 2>/dev/null; or break
        sleep 0.1
    end
    test "$observed" -eq 1; or begin; event observer_start_failed; exit 1; end
    event observer_sample_ready
end
set -l parent (sudo awk '/^PPid:/ {print $2}' "/proc/$perf_pid/status"); or exit 1
set -l start (sudo awk '{print $22}' "/proc/$perf_pid/stat"); or exit 1
set -l grandparent (sudo awk '/^PPid:/ {print $2}' "/proc/$parent/status"); or exit 1
set -l recorder_parent (sudo awk '/^PPid:/ {print $2}' "/proc/$recorder/status"); or exit 1
set -l recorder_now (sudo awk '{print $22}' "/proc/$recorder/stat"); or exit 1
test "$recorder_parent" = "$fish_pid"; and test "$recorder_now" = "$recorder_start"; or exit 1
begin; test "$parent" = "$recorder"; or test "$grandparent" = "$recorder"; end; and test "$start" = "$perf_start"; or exit 1
if kill -0 "$cleanup_job" 2>/dev/null; event cleanup_active_before_perf_stop; else; event cleanup_finished_before_perf_stop; end
event perf_stop_requested
sudo kill -INT "$perf_pid"; or exit 1
finish_profile_job "$recorder" "$recorder_start" recorder
set -l perf_exit $status
if test "$profile_ended" -eq 1; set -e recorder; end
if test "$perf_exit" -eq 0; event perf_stop_complete; else; event perf_stop_failed; exit 1; end
if kill -0 "$cleanup_job" 2>/dev/null; event cleanup_active_after_perf_stop; else; event cleanup_finished_after_perf_stop; end
for attempt in (seq 1 5)
    kill -0 "$cleanup_job" 2>/dev/null; or break
    sleep 1
end
if kill -0 "$cleanup_job" 2>/dev/null
    event sampler_stop_requested
    touch "$raw/sampler.stop"
    finish_profile_job "$sampler" "$sampler_start" sampler
    set -l sampler_exit $status
    if test "$profile_ended" -eq 1; set -e sampler; end
    if test "$sampler_exit" -eq 0; event sampler_stop_complete; else; event sampler_stop_failed; exit 1; end
end
for attempt in (seq 1 20)
    kill -0 "$cleanup_job" 2>/dev/null; or break
    sleep 1
end
if kill -0 "$cleanup_job" 2>/dev/null
    event instrumented_cleanup_incomplete
    terminate_owned_cleanup; or begin; event cleanup_termination_unverified; set -e cleanup_job; exit 1; end
    set -e cleanup_job
    exit 1
end
wait "$cleanup_job"
record_result "$raw/instrumented.result" instrumented_cleanup
child_result "$raw/instrumented.result"
set -l instrumented_exit $status
event "instrumented_cleanup_result=$instrumented_exit"
set -e cleanup_job
test -s "$raw/instrumented-stages"; or begin; event cleanup_stage_missing; exit 1; end
if test "$instrumented_exit" -eq 0; event instrumented_cleanup_complete; else; event instrumented_cleanup_failed; end
if set -q observer
    finish_profile_job "$observer" "$observer_start" observer 150
    set -l observer_exit $status
    if test "$profile_ended" -eq 1; set -e observer; end
    if test "$observer_exit" -eq 0; event observer_complete; else; event observer_failed; exit 1; end
end
if set -q sampler
    event sampler_stop_requested
    touch "$raw/sampler.stop"
    finish_profile_job "$sampler" "$sampler_start" sampler
    set -l sampler_exit $status
    if test "$profile_ended" -eq 1; set -e sampler; end
    if test "$sampler_exit" -eq 0; event sampler_stop_complete; else; event sampler_stop_failed; exit 1; end
end
test "$instrumented_exit" -eq 0; or exit 1
