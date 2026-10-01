command mkdir -p tmp/contract-tests
or exit $status
set -g selector_test_dir (command mktemp -d tmp/contract-tests/household-selector.XXXXXX)
or exit $status

function cleanup_household_selector_test --on-event fish_exit
    if string match -rq '^tmp/contract-tests/household-selector\.[A-Za-z0-9]{6}$' -- $selector_test_dir
        command rm -rf $selector_test_dir
    end
end

set -e HOUSEHOLD_COMPLETION_ACCEPTANCE
set -e HOUSEHOLD_TEST_FILE
set -e HOUSEHOLD_TEST_FILTER

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

echo 'Household completion selector preserves defaults, explicit selection and dashboard isolation order'
