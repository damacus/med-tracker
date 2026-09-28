set -l lane $argv[1]
set -l subnet $argv[2]
set -l run_dir $argv[3]
set -l failed

for mode in $argv[4..-1]
    set -l log "$run_dir/$mode.log"
    echo "$lane START $mode"
    if set -q CONTRACT_COMPLETE_DRY_RUN
        echo "CONTRACT_TEST_SUBNET=$subnet task api:$mode-acceptance" >$log
    else
        rtk proxy env CONTRACT_TEST_SUBNET=$subnet task api:$mode-acceptance >$log 2>&1
    end
    if test $status -eq 0
        echo "$lane PASS $mode"
    else
        set -a failed $mode
        echo "$lane FAIL $mode: $log" >&2
    end
end

if test (count $failed) -gt 0
    printf '%s\n' $failed >$run_dir/$lane.failed
    exit 1
end

rtk proxy touch $run_dir/$lane.passed
