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
set -l shared_namespace (string match -m 1 -r 'network_mode: service:rust-api' < rust/contract-tests/runner.compose.yaml)
set -l fixture_mount (string match -m 1 -r 'source: \$\{CONTRACT_FIXTURE_DIR\}' < rust/contract-tests/runner.compose.yaml)
set -l mailpit_endpoint (string match -m 1 -r 'CONTRACT_MAILPIT_URL: http://mail-test:8025' < rust/contract-tests/runner.compose.yaml)
set -l network_subnet (string match -r 'subnet: \$\{CONTRACT_TEST_SUBNET\}' < rust/contract-tests/runner-subnet.compose.yaml)
test (count $api_url $shared_namespace $fixture_mount $mailpit_endpoint $network_subnet) -eq 5
or begin; echo 'Compose runner lacks its internal network namespace or fixture bind' >&2; exit 1; end
set -l apns_key_mappings (string match -a -r 'APNS_PRIVATE_KEY: \$\{CONTRACT_APNS_PRIVATE_KEY:\?[^}]+\}' < rust/contract-tests/runner.compose.yaml)
test (count $apns_key_mappings) -eq 2
or begin; echo 'Compose runner must require a disposable APNs key for both API configurations' >&2; exit 1; end
if string match -rq -- '-----BEGIN PRIVATE KEY-----' < rust/contract-tests/runner.compose.yaml
    echo 'Compose runner contains a committed private key' >&2
    exit 1
end
set -l rails_read_namespace (string match -m 1 -r 'network_mode: service:web-test' < rust/contract-tests/runner.compose.yaml)
set -l rails_read_database (string match -m 1 -r 'CONTRACT_AUDIT_DATABASE_URL: postgresql://medtracker:medtracker_password@db-test:5432/medtracker_contract' < rust/contract-tests/runner.compose.yaml)
set -l rails_read_service (string match -m 1 -r '  rails-web-read-tests:' < rust/contract-tests/runner.compose.yaml)
test (count $rails_read_namespace $rails_read_database $rails_read_service) -eq 3
or begin; echo 'Rails read sidecar lacks shared web namespace or internal audit database' >&2; exit 1; end

fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set -l run_status $status
if test $run_status -ne 0
    cat $test_dir/output >&2
    echo "Medication Compose runner returned $run_status" >&2
    exit 1
end
if string match -rq -- '-----BEGIN PRIVATE KEY-----|CONTRACT_APNS_PRIVATE_KEY=' < $test_dir/output
    echo 'Runner printed its disposable APNs key' >&2
    exit 1
end
string match -rq 'Contract source SHA-256: [a-f0-9]{64}' < $test_dir/output
or begin; echo 'Runner omitted the immutable source digest' >&2; exit 1; end

set -l trace (cat $test_dir/trace)
contains -- api:contract-source-snapshot $trace
or begin; echo 'Runner did not freeze the Rust API build source' >&2; exit 1; end
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
if contains -- api:contract-browser-test $trace
    echo 'Runner launched browser checks without opting in' >&2
    exit 1
end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-source-snapshot
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set -l snapshot_status $status
test $snapshot_status -eq 42
or begin; cat $test_dir/output >&2; echo "Runner lost source snapshot failure status: $snapshot_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Runner skipped cleanup after source snapshot failure' >&2; exit 1; end
if contains -- api:contract-up $trace
    echo 'Runner launched API before source snapshot succeeded' >&2
    exit 1
end
set -e CONTRACT_FAKE_FAIL_STEP

set -lx CONTRACT_FAKE_FAIL_STEP apns_key_generation
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set -l key_generation_status $status
test $key_generation_status -eq 42
or begin; cat $test_dir/output >&2; echo "Runner lost APNs key generation failure status: $key_generation_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Runner skipped cleanup after APNs key generation failure' >&2; exit 1; end
if contains -- api:contract-up $trace
    echo 'Runner launched API without a generated APNs key' >&2
    exit 1
end
if string match -rq -- '-----BEGIN PRIVATE KEY-----|CONTRACT_APNS_PRIVATE_KEY=' < $test_dir/output
    echo 'Runner printed an APNs key after generation failure' >&2
    exit 1
end
set -e CONTRACT_FAKE_FAIL_STEP

set -lx CONTRACT_BROWSER_TESTS true
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
or begin; cat $test_dir/output >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- api:contract-browser-test $trace
or begin; echo 'Runner skipped opted-in browser checks' >&2; exit 1; end

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

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-browser-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails medication-read-api >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Runner lost browser failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Runner skipped cleanup after browser failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Runner skipped image cleanup after browser failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
set -lx CONTRACT_FAKE_REQUIRE_BROWSER_SNAPSHOT 1
set -lx BROWSER_TEST_FILES tests/household-completion-dashboard.test.mjs tests/household-completion-locales.test.mjs
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails browser-journey-rust >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Rust journey runner did not start the isolated API' >&2; exit 1; end
contains -- api:contract-ready $trace
or begin; echo 'Rust journey runner did not wait for API readiness' >&2; exit 1; end
contains -- api:contract-browser-rust $trace
or begin; echo 'Rust journey runner skipped its selected browser suite' >&2; exit 1; end
contains -- browser-source-snapshot-verified $trace
or begin; echo 'Rust journey browser build did not receive its captured Dockerfile and selected tests' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Rust journey runner skipped cleanup' >&2; exit 1; end
if contains -- api:contract-test $trace; or contains -- api:contract-browser-test $trace; or contains -- api:contract-browser-rails $trace
    echo 'Rust journey runner selected unrelated API, login or Rails browser tests' >&2
    exit 1
end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-browser-rust
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails browser-journey-rust >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Rust journey runner lost browser failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Rust journey runner skipped cleanup after browser failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Rust journey runner skipped image cleanup after browser failure' >&2; exit 1; end

set -lx PROFILE_ACCEPTANCE true
set -lx HOUSEHOLD_ACCEPTANCE true
set -lx BROWSER_TEST_FILES tests/settings-timezone.test.mjs tests/profile-notifications.test.mjs tests/profile-security.test.mjs tests/profile-advanced.test.mjs
set -lx CONTRACT_FAKE_FAIL_STEP api:contract-household-web-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails browser-journey-rust >$test_dir/output 2>&1
set failure_status $status
set trace (cat $test_dir/trace)
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Profile runner lost contract failure status: $failure_status" >&2; exit 1; end
contains -- api:contract-household-web-test $trace
or begin; echo 'Profile runner skipped grouped contracts' >&2; exit 1; end
contains -- api:contract-browser-rust $trace
or begin; echo 'Profile runner skipped browser diagnostics after contract failure' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Profile runner skipped cleanup after grouped contract failure' >&2; exit 1; end

set -e PROFILE_ACCEPTANCE HOUSEHOLD_ACCEPTANCE

set -e CONTRACT_FAKE_FAIL_STEP
set -e CONTRACT_FAKE_REQUIRE_BROWSER_SNAPSHOT BROWSER_TEST_FILES
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-sessions >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-sessions-test $trace
or begin; echo 'Sessions runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Sessions runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Sessions runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-sessions-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-sessions >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Sessions runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Sessions runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Sessions runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails browser-journey-rails >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
contains -- api:contract-browser-rails $trace
or begin; echo 'Runner skipped isolated Rails browser journey' >&2; exit 1; end
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
if contains -- api:contract-up $trace; or contains -- api:contract-test $trace
    echo 'Rails browser baseline unnecessarily started Rust acceptance' >&2
    exit 1
end
contains -- cleanup $trace
or begin; echo 'Rails browser runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-browser-rails
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails browser-journey-rails >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Runner lost Rails browser failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Runner skipped cleanup after Rails browser failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Runner skipped image cleanup after Rails browser failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
set -e CONTRACT_BROWSER_TESTS
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails web-session-api >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
contains -- api:contract-web-session-test $trace
or begin; echo 'Runner skipped isolated web session API tests' >&2; exit 1; end
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Web session runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_BROWSER_TESTS true
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails web-reads-api >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-web-reads-test $trace
or begin; echo 'Web reads runner skipped its selected HTTP target' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Web reads runner did not start the isolated Rust API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Web reads runner skipped cleanup' >&2; exit 1; end
if contains -- api:contract-test $trace; or contains -- api:contract-web-session-test $trace; or contains -- api:contract-browser-test $trace
    echo 'Web reads runner selected unrelated tests' >&2
    exit 1
end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-web-reads-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails web-reads-api >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Web reads runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Web reads runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Web reads runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP CONTRACT_BROWSER_TESTS
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails web-reads-rails >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-web-reads-rails-test $trace
or begin; echo 'Rails web reads runner skipped the isolated sidecar target' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Rails web reads runner skipped cleanup' >&2; exit 1; end
if contains -- api:contract-up $trace; or contains -- api:contract-test $trace; or contains -- api:contract-web-reads-test $trace; or contains -- run:web_reads_api:http://127.0.0.1:43017 $trace
    echo 'Rails web reads baseline started Rust acceptance' >&2
    exit 1
end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-web-reads-rails-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails web-reads-rails >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Rails web reads runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Rails web reads runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Rails web reads runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP CONTRACT_BROWSER_TESTS
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-dosages >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-dosages-test $trace
or begin; echo 'Dosage runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Dosage runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Dosage runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-dosages-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-dosages >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Dosage runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Dosage runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Dosage runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-people >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-people-test $trace
or begin; echo 'People runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'People runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'People runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-people-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-people >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "People runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'People runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'People runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-notifications >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-notifications-test $trace
or begin; echo 'Notification runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Notification runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Notification runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-notifications-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-notifications >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Notification runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Notification runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Notification runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-native-tokens >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-native-tokens-test $trace
or begin; echo 'Native token runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Native token runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Native token runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-native-tokens-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-native-tokens >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Native token runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Native token runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Native token runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-push-subscriptions >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-push-subscriptions-test $trace
or begin; echo 'Push subscription runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Push subscription runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Push subscription runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-push-subscriptions-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-push-subscriptions >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Push subscription runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Push subscription runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Push subscription runner skipped image cleanup after failure' >&2; exit 1; end

set -e CONTRACT_FAKE_FAIL_STEP
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-admin-settings >$test_dir/output 2>&1
set run_status $status
set trace (cat $test_dir/trace)
test $run_status -eq 0
or begin; cat $test_dir/output >&2; exit 1; end
contains -- api:contract-openapi-admin-settings-test $trace
or begin; echo 'Admin settings runner skipped its selected OpenAPI tests' >&2; exit 1; end
contains -- api:contract-up $trace
or begin; echo 'Admin settings runner did not start the isolated API' >&2; exit 1; end
contains -- cleanup $trace
or begin; echo 'Admin settings runner skipped cleanup' >&2; exit 1; end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-openapi-admin-settings-test
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-admin-settings >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "Admin settings runner lost test failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'Admin settings runner skipped cleanup after failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'Admin settings runner skipped image cleanup after failure' >&2; exit 1; end

for target in openapi-person-medication-writes openapi-schedule-writes openapi-pause-lifecycle openapi-dose-occurrences openapi-review-prompts openapi-app-tokens openapi-memberships openapi-stock-workflows openapi-audit-logs openapi-person-grants openapi-invitations openapi-invitations-legacy openapi-profile openapi-profile-storage openapi-read-completion openapi-rate-limit openapi-reports openapi-health-events openapi-exports openapi-sync-reads openapi-external-integrations openapi-portable-writes openapi-portability-legacy openapi-sync-batch-legacy openapi-sync-batch-replay-focus openapi-replay-legacy openapi-envelopes-legacy openapi-medications openapi-medications-focused api-legacy-auth api-legacy-admin api-legacy-care api-legacy-devices api-legacy-lookup
    set -e CONTRACT_FAKE_FAIL_STEP
    set -l selected_step (string join -- '' api:contract- $target -test)
    if test "$target" = openapi-invitations-legacy
        set selected_step api:contract-existing-invitations-test
    end
    command rm -f $test_dir/trace
    fish --no-config rust/contract-tests/run.fish rails $target >$test_dir/output 2>&1
    set run_status $status
    set trace (cat $test_dir/trace)
    test $run_status -eq 0
    or begin; cat $test_dir/output >&2; exit 1; end
    contains -- $selected_step $trace
    or begin; echo "$target runner skipped its selected OpenAPI tests" >&2; exit 1; end
    contains -- api:contract-up $trace
    or begin; echo "$target runner did not start the isolated API" >&2; exit 1; end
    if test "$target" = openapi-external-integrations
        contains -- api:contract-external-up $trace
        or begin; echo 'External runner skipped provider API listeners' >&2; exit 1; end
    end
    contains -- cleanup $trace
    or begin; echo "$target runner skipped cleanup" >&2; exit 1; end

    set -lx CONTRACT_FAKE_FAIL_STEP $selected_step
    command rm -f $test_dir/trace
    fish --no-config rust/contract-tests/run.fish rails $target >$test_dir/output 2>&1
    set failure_status $status
    test $failure_status -eq 42
    or begin; cat $test_dir/output >&2; echo "$target runner lost test failure status: $failure_status" >&2; exit 1; end
    set trace (cat $test_dir/trace)
    contains -- cleanup $trace
    or begin; echo "$target runner skipped cleanup after failure" >&2; exit 1; end
    contains -- api:contract-image-remove $trace
    or begin; echo "$target runner skipped image cleanup after failure" >&2; exit 1; end
end

set -lx CONTRACT_FAKE_FAIL_STEP api:contract-external-up
command rm -f $test_dir/trace
fish --no-config rust/contract-tests/run.fish rails openapi-external-integrations >$test_dir/output 2>&1
set failure_status $status
test $failure_status -eq 42
or begin; cat $test_dir/output >&2; echo "External runner lost provider startup failure status: $failure_status" >&2; exit 1; end
set trace (cat $test_dir/trace)
contains -- cleanup $trace
or begin; echo 'External runner skipped cleanup after provider startup failure' >&2; exit 1; end
contains -- api:contract-image-remove $trace
or begin; echo 'External runner skipped image cleanup after provider startup failure' >&2; exit 1; end

echo 'Medication, web reads, dosage, people, notification, native token, push subscription, admin settings, assignment, schedule, pause lifecycle and Rails browser runner sequences and failure cleanup passed'
