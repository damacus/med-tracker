command mkdir -p tmp/contract-tests
or exit $status
set -g test_dir (mktemp -d tmp/contract-tests/status.XXXXXX)
or exit $status
set -g shim_dir (mktemp -d /tmp/contract-fake-rtk.XXXXXX)
or exit $status
cp rust/contract-tests/test_support/rtk $shim_dir/rtk
or exit $status
chmod +x $shim_dir/rtk
or exit $status
set -g status_test_run_dirs

function cleanup_status_test --on-event fish_exit
    for run_dir in $status_test_run_dirs
        string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
        or continue
        if test -d $run_dir
            command rm -f $run_dir/owner $run_dir/fixture.json
            command rmdir $run_dir
        end
    end
    command rm -f $shim_dir/rtk $test_dir/latest-run $test_dir/output
    command rmdir $shim_dir $test_dir
end

set -lx PATH $shim_dir $PATH
set -lx CONTRACT_FAKE_RUN_DIR_FILE $test_dir/latest-run

function check_status -a run_status cleanup_status expected_status
    set -lx CONTRACT_FAKE_RUN_STATUS $run_status
    set -lx CONTRACT_FAKE_CLEANUP_STATUS $cleanup_status
    fish --no-config rust/contract-tests/run.fish rails > $test_dir/output 2>&1
    set -l actual_status $status
    set -l run_dir (cat $test_dir/latest-run)
    set -a status_test_run_dirs $run_dir
    test $actual_status -eq $expected_status
    or begin
        cat $test_dir/output >&2
        echo "Expected run=$run_status cleanup=$cleanup_status to exit $expected_status; got $actual_status" >&2
        return 1
    end
    if test $cleanup_status -eq 0
        test ! -e $run_dir/owner
        or begin; echo 'Successful cleanup retained the owner marker' >&2; return 1; end
    else
        test -f $run_dir/owner
        or begin; echo 'Failed cleanup lost the owner marker' >&2; return 1; end
    end
end

check_status 0 7 7
or exit $status
check_status 23 7 23
or exit $status
check_status 0 0 0
or exit $status
check_status 23 0 23
or exit $status

echo 'Contract run and cleanup exit statuses passed'
