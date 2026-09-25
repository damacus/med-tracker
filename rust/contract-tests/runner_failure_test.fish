command mkdir -p tmp/contract-tests
or exit $status
set -g test_dir (command mktemp -d tmp/contract-tests/runner-failure.XXXXXX)
or exit $status
set -g shim_dir (command mktemp -d /tmp/contract-fake-rtk.XXXXXX)
or exit $status
command cp rust/contract-tests/test_support/rtk $shim_dir/rtk
or exit $status
command chmod +x $shim_dir/rtk
or exit $status
command ln -s /usr/bin/true $shim_dir/sleep
or exit $status
set -g run_dirs

function cleanup_runner_failure_test --on-event fish_exit
    for run_dir in $run_dirs
        if string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
            if test -d $run_dir
                command rm -r $run_dir/storage
                command rm -f $run_dir/owner $run_dir/fixture.json
                command rmdir $run_dir
            end
        end
    end
    command rm -f $shim_dir/rtk $shim_dir/sleep $test_dir/latest-run $test_dir/output $test_dir/error $test_dir/trace
    command rmdir $shim_dir $test_dir
end

set -lx PATH $shim_dir $PATH
set -lx CONTRACT_FAKE_RUN_DIR_FILE $test_dir/latest-run
set -lx CONTRACT_FAKE_TRACE $test_dir/trace
set -lx CONTRACT_FAKE_REMOVE_OWNER 0
set -lx CONTRACT_FAKE_RUN_STATUS 0
set -lx CONTRACT_FAKE_CLEANUP_STATUS 0
set -lx CONTRACT_FAKE_FAIL_TARGET ''

function check_compose_port -a result expected_status expected_output
    set -lx CONTRACT_FAKE_COMPOSE_PORT_RESULT $result
    fish --no-config scripts/compose_port.fish mtcontract-1234567890abcdef test web-test 3000 >$test_dir/output 2>$test_dir/error
    set -l actual_status $status
    test $actual_status -eq $expected_status
    or begin; cat $test_dir/output >&2; echo "Compose port $result returned $actual_status" >&2; return 1; end
    if test -n "$expected_output"
        test (cat $test_dir/output) = "$expected_output"
        or begin; cat $test_dir/output >&2; echo "Compose port $result returned wrong output" >&2; return 1; end
    end
    return 0
end

check_compose_port 127.0.0.1:43017 0 43017
or exit $status
check_compose_port error 27 ''
or exit $status
check_compose_port empty 1 ''
or exit $status
set -e CONTRACT_FAKE_COMPOSE_PORT_RESULT

function check_failure -a fail_step fail_at expected_message
    set -lx CONTRACT_FAKE_FAIL_STEP $fail_step
    set -lx CONTRACT_FAKE_FAIL_AT $fail_at
    command rm -f $test_dir/trace
    fish --no-config rust/contract-tests/run.fish rails >$test_dir/output 2>&1
    set -l actual_status $status
    set -a run_dirs (cat $test_dir/latest-run)
    test $actual_status -ne 0
    or begin; cat $test_dir/output >&2; echo "$fail_step unexpectedly passed" >&2; return 1; end
    rg -q "$expected_message" $test_dir/output
    or begin; cat $test_dir/output >&2; echo "$fail_step lacked a diagnostic" >&2; return 1; end
    string match -q cleanup < $test_dir/trace
    or begin; cat $test_dir/trace >&2; echo "$fail_step skipped cleanup" >&2; return 1; end
    if test $fail_at -eq 1
        string match -q 'run:*' < $test_dir/trace
        and begin; cat $test_dir/trace >&2; echo "$fail_step used an invalid initial port" >&2; return 1; end
    else
        string match -q 'run:admin:*' < $test_dir/trace
        and begin; cat $test_dir/trace >&2; echo "$fail_step ran admin after failure" >&2; return 1; end
    end
    return 0
end

check_failure restart 2 'restart'
or exit $status
check_failure port_exit 2 'port'
or exit $status
check_failure port_empty 2 'port'
or exit $status
check_failure port_text 2 'port'
or exit $status
check_failure port_invalid 2 'port'
or exit $status
check_failure port_empty 1 'port'
or exit $status
check_failure ready 2 'ready'
or exit $status

echo 'Contract restart, port, and readiness failures stop before the next target'
