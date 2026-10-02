set -l subnet_a 10.252.122.0/28
set -l subnet_b 10.252.124.0/28
if set -q CONTRACT_COMPLETE_SUBNET_A
    set subnet_a $CONTRACT_COMPLETE_SUBNET_A
end
if set -q CONTRACT_COMPLETE_SUBNET_B
    set subnet_b $CONTRACT_COMPLETE_SUBNET_B
end
if test "$subnet_a" = "$subnet_b"
    echo 'Complete API acceptance requires two distinct subnets' >&2
    exit 2
end

if not set -q CONTRACT_COMPLETE_DRY_RUN
    rtk task api:contract-subnet-check CONTRACT_TEST_SUBNET=$subnet_a
    or exit $status
    rtk task api:contract-subnet-check CONTRACT_TEST_SUBNET=$subnet_b
    or exit $status
end

rtk proxy mkdir -p tmp/contract-tests
or exit $status
set -l run_dir (rtk proxy mktemp -d tmp/contract-tests/complete-api.XXXXXX)
or exit $status
echo "Complete API acceptance logs: $run_dir"

fish --no-config rust/contract-tests/run_complete_lane.fish a $subnet_a $run_dir \
    openapi-locations \
    openapi-person-medication-writes \
    openapi-pause-lifecycle \
    openapi-review-prompts \
    openapi-memberships \
    openapi-audit-logs \
    openapi-invitations \
    openapi-profile \
    openapi-reports \
    openapi-exports \
    openapi-portability-legacy \
    openapi-replay-legacy \
    openapi-read-completion \
    openapi-notifications \
    openapi-push-subscriptions \
    openapi-people \
    openapi-sessions \
    api-legacy-admin \
    api-legacy-lookup &
set -l lane_a_pid $last_pid

fish --no-config rust/contract-tests/run_complete_lane.fish b $subnet_b $run_dir \
    openapi-admin-settings \
    openapi-schedule-writes \
    openapi-dose-occurrences \
    openapi-app-tokens \
    openapi-stock-workflows \
    openapi-person-grants \
    openapi-invitations-legacy \
    openapi-profile-storage \
    openapi-rate-limit \
    openapi-health-events \
    openapi-external-integrations \
    openapi-portable-writes \
    openapi-sync-batch-legacy \
    openapi-envelopes-legacy \
    openapi-medications \
    openapi-native-tokens \
    openapi-dosages \
    api-legacy-auth \
    api-legacy-care \
    api-legacy-devices &
set -l lane_b_pid $last_pid

wait $lane_a_pid $lane_b_pid

if not test -f $run_dir/a.passed; or not test -f $run_dir/b.passed
    if test -f $run_dir/a.failed
        echo 'Lane A failures:' >&2
        cat $run_dir/a.failed >&2
    end
    if test -f $run_dir/b.failed
        echo 'Lane B failures:' >&2
        cat $run_dir/b.failed >&2
    end
    echo "Complete API acceptance failed; logs: $run_dir" >&2
    exit 1
end

echo "Complete API acceptance passed; logs: $run_dir"
