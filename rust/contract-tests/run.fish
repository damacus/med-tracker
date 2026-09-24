set -l mode $argv[1]

function remove_contract_fixture --on-event fish_exit
    if set -q contract_run_dir
        rtk proxy rm -f "$contract_fixture_path"
        rtk proxy rmdir "$contract_run_dir"
    end
end

if test "$mode" != rails; and test "$mode" != rust
    echo 'Expected rails or rust mode' >&2
    exit 2
end

rtk proxy mkdir -p tmp/contract-tests
or exit $status
set -g contract_run_dir (rtk proxy mktemp -d tmp/contract-tests/run.XXXXXX)
or exit $status
set -g contract_fixture_path (pwd)/$contract_run_dir/fixture.json

set -lx CONTRACT_RATE_LIMITING true
set -lx CONTRACT_DATABASE_URL postgresql://medtracker:medtracker_password@db-test:5432/medtracker_contract
rtk task contract:prepare-db
or exit $status
rtk task test:server
or exit $status

rtk task --force test:exec CMD="CONTRACT_FIXTURE_PATH=/app/$contract_run_dir/fixture.json rails runner scripts/contract_provision.rb"
or exit $status

if test "$mode" = rails
    set -l port (rtk task test:port)
    or exit $status
    if test "$argv[2]" = oauth
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
    else if test "$argv[2]" = care
        rtk task contract:run-care BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
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
    else
        rtk task contract:run BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
    end
else
    rtk task contract:run BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
end
