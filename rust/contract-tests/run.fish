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
        if set -q contract_api_image
            rtk task api:contract-image-remove CONTRACT_PROJECT=$contract_project
            or return $status
        end
    end
    if test -d "$contract_run_dir/storage"
        rtk proxy rm -r "$contract_run_dir/storage"
        or return $status
        echo "Contract storage removed: $contract_run_dir/storage"
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

function contract_web_port -a project
    set -l port (rtk task test:port CONTRACT_PROJECT=$project)
    or begin
        echo "Contract web port lookup failed for $project" >&2
        return 1
    end
    if test (count $port) -ne 1; or not string match -rq '^[0-9]{1,5}$' -- $port
        echo "Invalid contract web port for $project: $port" >&2
        return 1
    end
    if test $port -lt 1; or test $port -gt 65535
        echo "Invalid contract web port for $project: $port" >&2
        return 1
    end
    echo $port
end

function run_rails_contract_targets -a base_url fixture_path mailpit_url project run_dir
    set -l failed_targets
    set -l targets auth admin invitations care medication_stock dosage_health schedules assignments doses reviews reports fhir smart_fhir platform lookup web_json_read portability retained web_json_actions profile web_profile sync replay oauth devices web_devices push_delivery mcp uploads envelopes
    rtk task contract:run BASE_URL="$base_url" FIXTURE_PATH="$fixture_path" MAILPIT_URL="$mailpit_url" TEST_TARGET=lib
    or set -a failed_targets lib
    set -l previous_target
    for target in $targets
        if test "$target" = lookup
            set_lookup_adapter_environment
        else if uses_web_csrf $target
            clear_contract_adapter_environment
            set_web_device_environment
        else if test "$target" = web_json_read
            set_web_json_read_adapter_environment
        else if test "$target" = web_json_actions
            clear_contract_adapter_environment
            set_web_json_adapter_environment
        else if test "$target" = push_delivery
            clear_contract_adapter_environment
            set_push_delivery_environment
        else
            clear_contract_adapter_environment
        end
        if test "$target" != auth
            if contains -- "$target" lookup web_json_read web_json_actions push_delivery; or contains -- "$previous_target" lookup web_json_read web_json_actions push_delivery; or uses_web_csrf $target; or uses_web_csrf $previous_target
                rtk task test:server CONTRACT_PROJECT=$project
            else
                rtk task contract:restart-web CONTRACT_PROJECT=$project CONTRACT_RUN_DIR=$run_dir
            end
            or begin
                set -l restart_status $status
                echo "Contract web restart failed before $target" >&2
                return $restart_status
            end
            set -l port (contract_web_port $project)
            or return $status
            set base_url "http://127.0.0.1:$port"
            wait_for_contract_web $base_url
            or return $status
        end
        set previous_target $target
        rtk task contract:run BASE_URL="$base_url" FIXTURE_PATH="$fixture_path" MAILPIT_URL="$mailpit_url" TEST_TARGET=$target
        or set -a failed_targets $target
    end
    set -gx CONTRACT_AI_MEDICATION_HELP_ENABLED false
    set_web_json_adapter_environment
    rtk task test:server CONTRACT_PROJECT=$project
    or return $status
    set -l port (contract_web_port $project)
    or return $status
    set base_url "http://127.0.0.1:$port"
    wait_for_contract_web $base_url
    or return $status
    rtk task contract:run-web-json-actions-disabled BASE_URL="$base_url" FIXTURE_PATH="$fixture_path"
    or set -a failed_targets web_json_actions_disabled
    if test (count $failed_targets) -gt 0
        echo "Contract targets failed: "(string join ', ' $failed_targets) >&2
        return 1
    end
end

function set_lookup_adapter_environment
    set -gx CONTRACT_NHS_DMD_CLIENT_ID contract-id
    set -gx CONTRACT_NHS_DMD_CLIENT_SECRET contract-secret
    set -gx CONTRACT_RUBYOPT -r/app/rust/contract-tests/test_support/nhs_dmd_webmock
end

function set_web_device_environment
    set -gx CONTRACT_RUBYOPT -r/app/rust/contract-tests/test_support/csrf
end

function set_web_json_read_adapter_environment
    set_lookup_adapter_environment
    set -gx CONTRACT_RUBYOPT '-r/app/rust/contract-tests/test_support/nhs_dmd_webmock -r/app/rust/contract-tests/test_support/web_json_read_webmock'
end

function set_web_json_adapter_environment
    set -gx CONTRACT_RUBYOPT '-r/app/rust/contract-tests/test_support/ai_suggestion_adapter -r/app/rust/contract-tests/test_support/csrf'
end

function uses_web_csrf -a target
    contains -- $target platform web_profile web_devices retained
end

function set_push_delivery_environment
    set -gx CONTRACT_RUBYOPT '-r/app/rust/contract-tests/test_support/csrf -r/app/rust/contract-tests/test_support/push_delivery_adapter'
end

function clear_contract_adapter_environment
    set -e CONTRACT_NHS_DMD_CLIENT_ID CONTRACT_NHS_DMD_CLIENT_SECRET CONTRACT_RUBYOPT
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
    rtk proxy mkdir -p "$contract_run_dir/storage"
    or return $status
    set -gx CONTRACT_STORAGE_ROOT (rtk proxy realpath "$contract_run_dir/storage")
    set -gx COMPOSE_FILE compose.yaml:rust/contract-tests/storage.compose.yaml
    if set -q CONTRACT_TEST_SUBNET; and test -n "$CONTRACT_TEST_SUBNET"
        rtk task api:contract-subnet-check CONTRACT_TEST_SUBNET=$CONTRACT_TEST_SUBNET
        or return $status
        set -gx COMPOSE_FILE "$COMPOSE_FILE:rust/contract-tests/runner-subnet.compose.yaml"
    end
    if contains -- "$argv[2]" medication-read-api web-session-api web-reads-api browser-journey-rails web-reads-rails
        set -gx CONTRACT_AUTH_SESSION_SECRET (rtk proxy openssl rand -hex 32)
        or return $status
        set -gx COMPOSE_FILE "$COMPOSE_FILE:rust/contract-tests/runner.compose.yaml"
        set -gx CONTRACT_FIXTURE_DIR (rtk proxy realpath "$contract_run_dir")
    end
    set -lx CONTRACT_PROJECT $contract_project
    echo "Contract run project: $contract_project"

    set -lx CONTRACT_RATE_LIMITING true
    if test "$argv[2]" = web-json-actions-disabled
        set -gx CONTRACT_AI_MEDICATION_HELP_ENABLED false
    else
        set -gx CONTRACT_AI_MEDICATION_HELP_ENABLED true
    end
    if test "$argv[2]" = lookup
        set_lookup_adapter_environment
    else if contains -- "$argv[2]" platform web-profile web-devices retained browser-journey-rails
        set_web_device_environment
    else if test "$argv[2]" = web_json_read
        set_web_json_read_adapter_environment
    else if test "$argv[2]" = web-json-actions; or test "$argv[2]" = web-json-actions-disabled
        set_web_json_adapter_environment
    else if test "$argv[2]" = push-delivery
        set_push_delivery_environment
    else
        clear_contract_adapter_environment
    end
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

    if test "$argv[2]" = browser-journey-rails
        set -g contract_api_image true
        rtk task api:contract-browser-rails CONTRACT_PROJECT=$contract_project
        return $status
    end

    if test "$argv[2]" = web-reads-rails
        set -g contract_api_image true
        rtk task api:contract-web-reads-rails-test CONTRACT_PROJECT=$contract_project
        return $status
    end

    if contains -- "$argv[2]" medication-read-api web-session-api web-reads-api
        set -g contract_api_image true
        rtk task api:contract-up CONTRACT_PROJECT=$contract_project
        or return $status
        rtk task api:contract-ready CONTRACT_PROJECT=$contract_project
        or return $status
        if test "$argv[2]" = web-session-api
            rtk task api:contract-web-session-test CONTRACT_PROJECT=$contract_project
        else if test "$argv[2]" = web-reads-api
            rtk task api:contract-web-reads-test CONTRACT_PROJECT=$contract_project
        else
            rtk task api:contract-test CONTRACT_PROJECT=$contract_project
        end
        or return $status
        if test "$argv[2]" != web-reads-api; and test "$CONTRACT_BROWSER_TESTS" = true
            rtk task api:contract-browser-test CONTRACT_PROJECT=$contract_project
            or return $status
        end
        return 0
    end

    set -l mail_port (rtk task contract:mail-port CONTRACT_PROJECT=$contract_project)
    or return $status
    set -l mailpit_url "http://127.0.0.1:$mail_port"
    rtk task contract:mail-clear MAILPIT_URL="$mailpit_url"
    or return $status

    if test "$mode" = rails
        set -l port (contract_web_port $contract_project)
        or return $status
        if test "$argv[2]" = admin
            rtk task contract:run-admin BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = auth-boundaries
            rtk task contract:run-auth-boundaries BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = auth-session-boundary
            rtk task contract:run-auth-session-boundary BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = admin-tokens-audit
            rtk task contract:run-admin-tokens-audit BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = invitations
            rtk task contract:run-invitations BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path" MAILPIT_URL="$mailpit_url"
        else if test "$argv[2]" = devices
            rtk task contract:run-devices BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
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
        else if test "$argv[2]" = fhir
            rtk task contract:run-fhir BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = smart-fhir
            rtk task contract:run-smart-fhir BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = platform
            rtk task contract:run-platform BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = lookup
            rtk task contract:run-lookup BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = web_json_read
            rtk task contract:run-web-json-read BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = portability
            rtk task contract:run-portability BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = retained
            rtk task contract:run-retained BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = web-json-actions
            rtk task contract:run-web-json-actions BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = web-json-actions-disabled
            rtk task contract:run-web-json-actions-disabled BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = profile
            rtk task contract:run-profile BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = profile-web-profile
            rtk task contract:run-profile BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
            or return $status
            set_web_device_environment
            rtk task test:server CONTRACT_PROJECT=$contract_project
            or return $status
            set port (contract_web_port $contract_project)
            or return $status
            wait_for_contract_web "http://127.0.0.1:$port"
            or return $status
            rtk task contract:run-web-profile BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = web-profile
            rtk task contract:run-web-profile BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = mcp
            rtk task contract:run-mcp BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = uploads
            rtk task contract:run-uploads BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = web-devices
            rtk task contract:run-web-devices BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
        else if test "$argv[2]" = push-delivery
            rtk task contract:run-push-delivery BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$contract_fixture_path"
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
        else if test "$argv[2]" = auth-boundaries
            rtk task contract:run-auth-boundaries BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = auth-session-boundary
            rtk task contract:run-auth-session-boundary BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = admin-tokens-audit
            rtk task contract:run-admin-tokens-audit BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = invitations
            rtk task contract:run-invitations BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN" MAILPIT_URL="$mailpit_url"
        else if test "$argv[2]" = oauth
            rtk task contract:oauth BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = devices
            rtk task contract:run-devices BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = sync
            rtk task contract:run-sync BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = replay
            rtk task contract:run-replay BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = reports
            rtk task contract:run-reports BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = fhir
            rtk task contract:run-fhir BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = smart-fhir
            rtk task contract:run-smart-fhir BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = platform
            rtk task contract:run-platform BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = lookup
            rtk task contract:run-lookup BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = web_json_read
            rtk task contract:run-web-json-read BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = portability
            rtk task contract:run-portability BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = retained
            rtk task contract:run-retained BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = web-json-actions
            rtk task contract:run-web-json-actions BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = web-json-actions-disabled
            rtk task contract:run-web-json-actions-disabled BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = profile
            rtk task contract:run-profile BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = web-profile
            rtk task contract:run-web-profile BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = mcp
            rtk task contract:run-mcp BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = uploads
            rtk task contract:run-uploads BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = web-devices
            rtk task contract:run-web-devices BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
        else if test "$argv[2]" = push-delivery
            rtk task contract:run-push-delivery BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$contract_fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
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
