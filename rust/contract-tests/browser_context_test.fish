rtk proxy mkdir -p tmp/contract-tests
or exit $status
set -g browser_context_test_dir (rtk proxy mktemp -d tmp/contract-tests/browser-context.XXXXXX)
or exit $status

function cleanup_browser_context_test --on-event fish_exit
    if string match -rq '^tmp/contract-tests/browser-context\.[A-Za-z0-9]{6}$' -- "$browser_context_test_dir"
        rtk proxy rm -rf -- "$browser_context_test_dir"
    end
end

set -l workspace (pwd)
set -lx MEDTRACKER_GIT_COMMON_DIR (rtk git rev-parse --path-format=absolute --git-common-dir)
or exit $status
set -lx COMPOSE_FILE compose.yaml:rust/contract-tests/storage.compose.yaml:rust/contract-tests/runner.compose.yaml
set -lx CONTRACT_PROJECT mtcontract-browser-context-$fish_pid
set -lx CONTRACT_FIXTURE_DIR "$workspace/$browser_context_test_dir"
set -lx CONTRACT_STORAGE_ROOT "$CONTRACT_FIXTURE_DIR/storage"
set -lx CONTRACT_AUTH_SESSION_SECRET (rtk proxy openssl rand -hex 32)
or exit $status
set -lx CONTRACT_RODAUTH_HMAC_SECRET (rtk proxy openssl rand -hex 32)
or exit $status
set -lx CONTRACT_APNS_PRIVATE_KEY (rtk proxy openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:prime256v1 | string collect)
if test $status -ne 0; or test (count $CONTRACT_APNS_PRIVATE_KEY) -ne 1
    echo 'Disposable context-test signing key generation failed' >&2
    exit 1
end

rtk proxy mkdir -p "$CONTRACT_STORAGE_ROOT" "$CONTRACT_FIXTURE_DIR/captured/rust/web/tests"
or exit $status
rtk proxy cp rust/web/Dockerfile.smoke "$CONTRACT_FIXTURE_DIR/captured/rust/web/Dockerfile.smoke"
or exit $status
for browser_test in household-completion-dashboard.test.mjs household-completion-locales.test.mjs
    rtk proxy cp "rust/web/tests/$browser_test" "$CONTRACT_FIXTURE_DIR/captured/rust/web/tests/$browser_test"
    or exit $status
end

function assert_browser_contexts --argument-names scenario expected_rust expected_rails
    rtk proxy docker compose -p "$CONTRACT_PROJECT" --profile test config --format json | jq -er '.services["rust-browser-tests"].build.context, .services["rails-browser-tests"].build.context' >"$browser_context_test_dir/contexts"
    set -l render_status $pipestatus
    if test $render_status[1] -ne 0; or test $render_status[2] -ne 0
        echo "Could not resolve browser build contexts for $scenario" >&2
        return 1
    end
    set -l contexts (rtk proxy cat "$browser_context_test_dir/contexts")
    test (count $contexts) -eq 2
    or begin; echo "Expected two browser contexts for $scenario" >&2; return 1; end
    test "$contexts[1]" = "$expected_rust"
    or begin; echo "Rust browser context differs in $scenario: $contexts[1]" >&2; return 1; end
    test "$contexts[2]" = "$expected_rails"
    or begin; echo "Rails browser context changed in $scenario: $contexts[2]" >&2; return 1; end
    test -f "$contexts[1]/Dockerfile.smoke"
    or begin; echo "Resolved Rust browser Dockerfile missing in $scenario" >&2; return 1; end
    for browser_test in household-completion-dashboard.test.mjs household-completion-locales.test.mjs
        test -f "$contexts[1]/tests/$browser_test"
        or begin; echo "Resolved Rust browser test missing in $scenario: $browser_test" >&2; return 1; end
    end
end

set -e CONTRACT_BROWSER_BUILD_CONTEXT
assert_browser_contexts unset "$workspace/rust/web" "$workspace/rust/web"
or exit $status
set -lx CONTRACT_BROWSER_BUILD_CONTEXT ''
assert_browser_contexts empty "$workspace/rust/web" "$workspace/rust/web"
or exit $status
set CONTRACT_BROWSER_BUILD_CONTEXT "$workspace/rust/web"
assert_browser_contexts explicit-default "$workspace/rust/web" "$workspace/rust/web"
or exit $status
set CONTRACT_BROWSER_BUILD_CONTEXT "$CONTRACT_FIXTURE_DIR/captured/rust/web"
assert_browser_contexts captured "$CONTRACT_FIXTURE_DIR/captured/rust/web" "$workspace/rust/web"
or exit $status

echo 'Merged Compose browser contexts preserve default and captured Rust inputs and the Rails baseline'
