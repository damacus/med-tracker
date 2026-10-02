command mkdir -p tmp/contract-tests
or exit $status
set -g selector_test_dir (command mktemp -d tmp/contract-tests/household-selector.XXXXXX)
or exit $status

function cleanup_household_selector_test --on-event fish_exit
    if string match -rq '^tmp/contract-tests/household-selector\.[A-Za-z0-9]{6}$' -- $selector_test_dir
        command rm -rf $selector_test_dir
    end
end

set -e HOUSEHOLD_ACCEPTANCE
set -e HOUSEHOLD_COMPLETION_ACCEPTANCE
set -e HOUSEHOLD_STOCK_ACCEPTANCE
set -e HOUSEHOLD_TEST_FILE
set -e HOUSEHOLD_TEST_FILTER
set -e BROWSER_TEST_FILES

function assert_household_command
    set -l case_name $argv[1]
    set -l expected_command $argv[2]
    set -l task_arguments $argv
    set -e task_arguments[1..2]
    rtk proxy task --dry --verbose api:contract-household-web-test CONTRACT_PROJECT=mtcontract-selector-probe $task_arguments >$selector_test_dir/rendered 2>&1
    set -l render_status $status
    if test $render_status -ne 0
        cat $selector_test_dir/rendered >&2
        echo "Household Task render failed for $case_name: $render_status" >&2
        return 1
    end
    set -l rendered_command (string match -r 'cargo test --locked --manifest-path rust/contract-tests/Cargo.toml .*' < $selector_test_dir/rendered)
    set -l expected_commands (string split \n -- "$expected_command")
    if test (count $rendered_command) -ne (count $expected_commands)
        cat $selector_test_dir/rendered >&2
        echo "Household Task Cargo command count differs for $case_name: expected "(count $expected_commands)", actual "(count $rendered_command) >&2
        return 1
    end
    set -l command_index 1
    for expected in $expected_commands
        set -l actual (string replace -ra '[[:space:]]+' ' ' -- $rendered_command[$command_index])
        if test "$actual" != "$expected"
            echo "Household Cargo selection differs for $case_name command $command_index" >&2
            echo "Expected: $expected" >&2
            echo "Actual: $actual" >&2
            return 1
        end
        set command_index (math $command_index + 1)
    end
end

set -l cargo_prefix 'cargo test --locked --manifest-path rust/contract-tests/Cargo.toml'
set -l original_targets '--test household_web --test household_navigation --test household_lifecycle'
set -l unfiltered_suffix ' -- --test-threads=1'
set -l completion_commands "$cargo_prefix $original_targets --test household_completion_medication$unfiltered_suffix
$cargo_prefix --test household_completion_dashboard$unfiltered_suffix"

assert_household_command unset "$cargo_prefix $original_targets$unfiltered_suffix"
or exit $status
assert_household_command false "$cargo_prefix $original_targets$unfiltered_suffix" HOUSEHOLD_COMPLETION_ACCEPTANCE=false
or exit $status
assert_household_command true "$completion_commands" HOUSEHOLD_COMPLETION_ACCEPTANCE=true
or exit $status
assert_household_command explicit "$cargo_prefix --test household_completion_medication -- first_option_inserted_between_medication_and_options_reads_rejects_stale_scalar_draft_without_writes --test-threads=1" HOUSEHOLD_COMPLETION_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_completion_medication HOUSEHOLD_TEST_FILTER=first_option_inserted_between_medication_and_options_reads_rejects_stale_scalar_draft_without_writes
or exit $status

command mkdir -p $selector_test_dir/bin
or exit $status
set -l real_fish (command -s fish)
if test -z "$real_fish"
    echo 'Cannot find Fish for household wrapper probe' >&2
    exit 1
end
printf '#!%s\n' "$real_fish" >$selector_test_dir/bin/fish
or exit $status
command cat rust/contract-tests/fixtures/household_wrapper_probe.fish >>$selector_test_dir/bin/fish
or exit $status
command chmod +x $selector_test_dir/bin/fish
or exit $status

function assert_household_wrapper_environment
    set -l case_name $argv[1]
    set -l expected_values $argv[2..7]
    set -l task_arguments
    if set -q argv[8]
        set task_arguments $argv[8..-1]
    end
    set -lx PATH "$selector_test_dir/bin" $PATH
    set -lx SELECTOR_PROBE_RECEIPT "$selector_test_dir/actual-env"
    command rm -f $SELECTOR_PROBE_RECEIPT
    printf '%s\n' $expected_values >$selector_test_dir/expected-env
    rtk proxy task api:browser-rust $task_arguments >$selector_test_dir/wrapper-output 2>&1
    set -l wrapper_status $status
    if test $wrapper_status -ne 0
        cat $selector_test_dir/wrapper-output >&2
        echo "Household wrapper failed for $case_name: $wrapper_status" >&2
        return 1
    end
    if not command cmp -s $selector_test_dir/expected-env $SELECTOR_PROBE_RECEIPT
        echo "Household wrapper did not forward the requested tests for $case_name" >&2
        command diff -u $selector_test_dir/expected-env $SELECTOR_PROBE_RECEIPT >&2
        return 1
    end
end

set -l default_browser_files 'tests/medication-journey.test.mjs tests/medication-journey-form.test.mjs'
assert_household_wrapper_environment defaults '' '' '' '' '' "$default_browser_files"
or exit $status
assert_household_wrapper_environment explicit true true true household_treatments direct_assignment_creates_edits_and_keeps_invalid_and_stale_drafts tests/household-treatments.test.mjs HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_COMPLETION_ACCEPTANCE=true HOUSEHOLD_STOCK_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatments HOUSEHOLD_TEST_FILTER=direct_assignment_creates_edits_and_keeps_invalid_and_stale_drafts BROWSER_TEST_FILES=tests/household-treatments.test.mjs
or exit $status
assert_household_wrapper_environment false false false false '' '' "$default_browser_files" HOUSEHOLD_ACCEPTANCE=false HOUSEHOLD_COMPLETION_ACCEPTANCE=false HOUSEHOLD_STOCK_ACCEPTANCE=false
or exit $status

begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_treatments
    set -lx HOUSEHOLD_TEST_FILTER assignment_pause_and_resume_preserve_reason_history_and_replay
    assert_household_wrapper_environment exported true '' '' household_treatments assignment_pause_and_resume_preserve_reason_history_and_replay "$default_browser_files"
    or exit $status
end

begin
    set -lx HOUSEHOLD_ACCEPTANCE false
    set -lx HOUSEHOLD_TEST_FILE household_stock
    set -lx BROWSER_TEST_FILES tests/medication-journey.test.mjs
    assert_household_wrapper_environment cli_over_export true '' '' household_treatments '' tests/household-treatments.test.mjs HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatments BROWSER_TEST_FILES=tests/household-treatments.test.mjs
    or exit $status
end

set -l quoted_filter "literal filter with spaces; \"quoted\" and 'apostrophe'"
assert_household_wrapper_environment quoted_filter true '' '' household_treatments "$quoted_filter" "$default_browser_files" HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatments "HOUSEHOLD_TEST_FILTER=$quoted_filter"
or exit $status

echo 'Household selectors preserve defaults, forward requested tests and keep dashboard isolation order'
