set -l mode $argv[1]

function cleanup_contract_run
    if not set -q contract_run_dir
        return 0
    end
    if set -q contract_project
        if not test -f "$contract_run_dir/owner"
            echo "Contract project cleanup blocked: ownership marker missing at $contract_run_dir/owner for $contract_project; run directory retained" >&2
            return 1
        end
        rtk task contract:cleanup CONTRACT_PROJECT=$contract_project CONTRACT_RUN_DIR=$contract_run_dir
        or begin
            set -l cleanup_status $status
            echo "Contract project cleanup failed; ownership marker retained at $contract_run_dir/owner" >&2
            return $cleanup_status
        end
    end
    rtk proxy rm -f "$contract_fixture_path" "$contract_run_dir/owner"
    or return $status
    rtk proxy rmdir "$contract_run_dir"
end

function cleanup_contract_on_exit --on-event fish_exit
    if set -q contract_cleanup_pending
        cleanup_contract_run
    end
end

function wait_for_contract_web -a base_url
    for attempt in (seq 30)
        rtk proxy curl --fail --silent --show-error --max-time 2 --output /dev/null "$base_url/up" 2>/dev/null
        and return 0
        sleep 1
    end
    echo "Contract web server did not become ready at $base_url/up" >&2
    return 1
end

function run_rails_contract_targets -a base_url fixture_path mailpit_url project run_dir
    set -l failed_targets
    set -l targets auth admin invitations care medication_stock dosage_health schedules assignments doses reviews reports portability sync replay oauth envelopes
    rtk task contract:run BASE_URL="$base_url" FIXTURE_PATH="$fixture_path" MAILPIT_URL="$mailpit_url" TEST_TARGET=lib
    or set -a failed_targets lib
    for target in $targets
        if test "$target" != auth
            rtk task contract:restart-web CONTRACT_PROJECT=$project CONTRACT_RUN_DIR=$run_dir
            or return $status
            set -l port (rtk task test:port CONTRACT_PROJECT=$project)
            or return $status
            set base_url "http://127.0.0.1:$port"
            wait_for_contract_web $base_url
            or return $status
        end
        rtk task contract:run BASE_URL="$base_url" FIXTURE_PATH="$fixture_path" MAILPIT_URL="$mailpit_url" TEST_TARGET=$target
        or set -a failed_targets $target
    end
    if test (count $failed_targets) -gt 0
        echo "Contract targets failed: "(string join ', ' $failed_targets) >&2
        return 1
    end
end

if test "$mode" != rails; and test "$mode" != rust
    echo 'Expected rails or rust mode' >&2
    exit 2
end

function run_contract
    set -l mode $argv[1]
    rtk proxy mkdir -p tmp/contract-tests
    or return $status
    set -g contract_run_dir (rtk proxy mktemp -d tmp/contract-tests/run.XXXXXX)
    or return $status
    set -g contract_fixture_path (pwd)/$contract_run_dir/fixture.json
    set -l generated_project mtcontract-(string lower (string replace -a - '' (uuidgen)) | string sub -l 16)
    string match -rq '^mtcontract-[a-f0-9]{16}$' -- $generated_project
    or return 2
    set -g contract_project $generated_project
    echo $contract_project >$contract_run_dir/owner
    or return $status
    set -g contract_cleanup_pending true
    set -lx CONTRACT_PROJECT $contract_project
    echo "Contract run project: $contract_project"

    set -lx CONTRACT_RATE_LIMITING true
    set -lx CONTRACT_DATABASE_URL postgresql://medtracker:medtracker_password@db-test:5432/medtracker_contract
    set -l startup_at (date +%s)
    rtk task contract:prepare-db CONTRACT_PROJECT=$contract_project
    or return $status
    rtk task test:server CONTRACT_PROJECT=$contract_project
    or return $status
    set -l server_seconds (math (date +%s) - $startup_at)
    echo "Contract server ready after $server_seconds seconds"

    rtk task --force test:exec CONTRACT_PROJECT=$contract_project CMD="CONTRACT_FIXTURE_PATH=/app/$contract_run_dir/fixture.json rails runner scripts/contract_provision.rb"
    or return $status
    set -l fixture_seconds (math (date +%s) - $startup_at)
    echo "Contract fixture ready after $fixture_seconds seconds"

    set -l mail_port (rtk task contract:mail-port CONTRACT_PROJECT=$contract_project)
    or return $status
    set -l mailpit_url "http://127.0.0.1:$mail_port"
    rtk task contract:mail-clear MAILPIT_URL="$mailpit_url"
    or return $status

    if test "$mode" = rails
        set -l port (rtk task test:port CONTRACT_PROJECT=$contract_project)
        or return $status
        if test "$argv[2]" = admin
            rtk task contract:run-admin BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = admin-tokens-audit
            rtk task contract:run-admin-tokens-audit BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = invitations
            rtk task contract:run-invitations BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path" MAILPIT_URL="$mailpit_url"
        else if test "$argv[2]" = oauth
            rtk task contract:oauth BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = dosage-health
            rtk task contract:run-dosage-health BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = schedules
            rtk task contract:run-schedules BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = assignments
            rtk task contract:run-assignments BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = doses
            rtk task contract:run-doses BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = reviews
            rtk task contract:run-reviews BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = reports
            rtk task contract:run-reports BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = portability
            rtk task contract:run-portability BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = care
            rtk task contract:run-care BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = sync
            rtk task contract:run-sync BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = replay
            rtk task contract:run-replay BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = sync-privacy
            rtk task contract:run-sync-privacy BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = dose-precision
            rtk task contract:run-dose-precision BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = assignment-precision
            rtk task contract:run-assignment-precision BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = schedule-precision
            rtk task contract:run-schedule-precision BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = dosage-health-privacy
            rtk task contract:run-dosage-health-privacy BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = dosage-health-default-validation
            rtk task contract:run-dosage-health-default-validation BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = health-event-kind-validation
            rtk task contract:run-health-event-kind-validation BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = health-event-replacement-atomicity
            rtk task contract:run-health-event-replacement-atomicity BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else
            run_rails_contract_targets "http://127.0.0.1:$port" "$contract_fixture_path" "$mailpit_url" $contract_project $contract_run_dir
        end
    else
        if test "$argv[2]" = admin
            rtk task contract:run-admin BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = admin-tokens-audit
            rtk task contract:run-admin-tokens-audit BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = invitations
            rtk task contract:run-invitations BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN" MAILPIT_URL="$mailpit_url"
        else if test "$argv[2]" = sync
            rtk task contract:run-sync BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = replay
            rtk task contract:run-replay BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = reports
            rtk task contract:run-reports BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = portability
            rtk task contract:run-portability BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else
            rtk task contract:run BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN" MAILPIT_URL="$mailpit_url"
        end
    end
end

run_contract $argv
set -l run_status $status
set -e contract_cleanup_pending
cleanup_contract_run
set -l cleanup_status $status
if test $run_status -ne 0
    exit $run_status
end
exit $cleanup_status
