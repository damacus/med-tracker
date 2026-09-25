command mkdir -p tmp/contract-tests
or exit $status
set -g test_dir (command mktemp -d tmp/contract-tests/runner.XXXXXX)
or exit $status
set -g shim_dir (command mktemp -d /tmp/contract-fake-rtk.XXXXXX)
or exit $status
command cp rust/contract-tests/test_support/rtk $shim_dir/rtk
or exit $status
command chmod +x $shim_dir/rtk
or exit $status
set -g run_dirs

function cleanup_runner_test --on-event fish_exit
    for run_dir in $run_dirs
        if string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
            command rm -r $run_dir/storage
            command rm -f $run_dir/owner $run_dir/fixture.json
            command rmdir $run_dir
        end
    end
    command rm -f $shim_dir/rtk $test_dir/latest-run $test_dir/output $test_dir/trace
    command rmdir $shim_dir $test_dir
end

set -lx PATH $shim_dir $PATH
set -lx CONTRACT_FAKE_RUN_DIR_FILE $test_dir/latest-run
set -lx CONTRACT_FAKE_TRACE $test_dir/trace
set -lx CONTRACT_FAKE_REMOVE_OWNER 0
set -lx CONTRACT_FAKE_RUN_STATUS 0
set -lx CONTRACT_FAKE_CLEANUP_STATUS 7
set -lx CONTRACT_FAKE_FAIL_TARGET schedules

fish --no-config rust/contract-tests/run.fish rails >$test_dir/output 2>&1
set -l actual_status $status
set -a run_dirs (cat $test_dir/latest-run)
test $actual_status -eq 1
or begin; cat $test_dir/output >&2; echo "Expected target failure to survive cleanup; got $actual_status" >&2; exit 1; end

set -l targets auth admin invitations care medication_stock dosage_health schedules assignments doses reviews reports portability profile sync replay oauth devices envelopes
set -l trace (cat $test_dir/trace)
set -l runs (string match 'run:*' -- $trace)
test (count $trace) -eq (math 4 x (count $targets)); and test "$trace[-1]" = cleanup
or begin; cat $test_dir/trace >&2; echo 'Full runner did not refresh each restarted target' >&2; exit 1; end
test (count $runs) -eq (math (count $targets) + 1)
or begin; cat $test_dir/trace >&2; echo 'Full runner did not run all targets' >&2; exit 1; end
test "$trace[1]" = port; and test "$trace[2]" = run:lib:http://127.0.0.1:43017
or begin; cat $test_dir/trace >&2; echo 'Full runner skipped library tests' >&2; exit 1; end
for index in (seq (count $targets))
    set -l run_index (math $index + 1)
    set -l expected_run "run:$targets[$index]:http://127.0.0.1:"(math 43016 + $index)
    test "$runs[$run_index]" = "$expected_run"
    or begin; cat $test_dir/trace >&2; echo "Wrong target at index $index" >&2; exit 1; end
    set -l trace_index (math 4 \* $index - 1)
    test "$trace[$trace_index]" = "$expected_run"
    or begin; cat $test_dir/trace >&2; echo "Runner did not isolate $targets[$index]" >&2; exit 1; end
    if test $index -lt (count $targets)
        set -l restart_index (math $trace_index + 1)
        set -l port_index (math $trace_index + 2)
        set -l ready_index (math $trace_index + 3)
        test "$trace[$restart_index]" = restart; and test "$trace[$port_index]" = port; and test "$trace[$ready_index]" = ready
        or begin; cat $test_dir/trace >&2; echo "No ready refreshed server after $targets[$index]" >&2; exit 1; end
    end
end
string match -q 'run:envelopes:*' -- $runs[-1]
or begin; echo 'A failed target skipped a later target' >&2; exit 1; end
rg -q 'schedules' $test_dir/output
or begin; echo 'Failed target was not reported' >&2; exit 1; end

echo 'Full contract target isolation and failure aggregation passed'
