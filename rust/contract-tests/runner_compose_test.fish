command mkdir -p tmp/contract-tests
or exit $status
set -g test_dir (command mktemp -d tmp/contract-tests/runner-compose.XXXXXX)
or exit $status
set -g shim_dir (command mktemp -d /tmp/contract-compose-rtk.XXXXXX)
or exit $status
command cp rust/contract-tests/test_support/rtk $shim_dir/rtk
or exit $status
command chmod +x $shim_dir/rtk
or exit $status

function cleanup_runner_compose_test --on-event fish_exit
    if test -f $test_dir/latest-run
        set -l run_dir (cat $test_dir/latest-run)
        if string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
            command rm -rf $run_dir
        end
    end
    command rm -rf $shim_dir $test_dir
end

set -lx PATH $shim_dir $PATH
set -lx CONTRACT_FAKE_RUN_DIR_FILE $test_dir/latest-run
set -lx CONTRACT_FAKE_TRACE $test_dir/trace
set -lx CONTRACT_FAKE_CLEANUP_STATUS 0
set -lx CONTRACT_FAKE_REQUIRE_RELATIVE_CLEANUP 1
set -lx CONTRACT_TEST_SUBNET 192.168.240.0/28

set -l api_url (string match -r 'CONTRACT_BASE_URL: http://127.0.0.1:39998' < rust/contract-tests/runner.compose.yaml)
set -l shared_namespace (string match -r 'network_mode: service:rust-api' < rust/contract-tests/runner.compose.yaml)
set -l fixture_mount (string match -r 'source: \$\{CONTRACT_FIXTURE_DIR\}' < rust/contract-tests/runner.compose.yaml)
set -l network_subnet (string match -r 'subnet: \$\{CONTRACT_TEST_SUBNET\}' < rust/contract-tests/runner-subnet.compose.yaml)
test (count $api_url $shared_namespace $fixture_mount $network_subnet) -eq 4
or begin; echo 'Compose runner lacks its internal network namespace or fixture bind' >&2; exit 1; end

fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set -l run_status $status
if test $run_status -ne 0
    cat $test_dir/output >&2
    echo "Medication Compose runner returned $run_status" >&2
    exit 1
end

set -l trace (cat $test_dir/trace)
contains -- api:contract-up $trace
or begin; echo 'Runner did not start the Compose API service' >&2; exit 1; end
contains -- api:contract-subnet-check $trace
or begin; echo 'Runner did not validate the optional test subnet' >&2; exit 1; end
contains -- api:contract-ready $trace
or begin; echo 'Runner did not wait for the Compose API service' >&2; exit 1; end
contains -- api:contract-test $trace
or begin; echo 'Runner did not run HTTP tests inside Compose' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Runner did not clean up its Compose project' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Runner did not remove its project image' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set -l failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Runner skipped project cleanup after test failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Runner skipped image cleanup after test failure' >&2; exit 1; end

echo 'Medication Compose runner task sequence and failure cleanup passed'
