set -l private_dir (path dirname (status filename))
set -l helper "$private_dir/browser_minor_wrapper.fish"
if set -q argv[1]
    set helper "$argv[1]"
end
set -l fake_task "$private_dir/fixtures/browser_minor_fake_task.fish"
set -l temporary (mktemp -d)
or exit 1
mkdir "$temporary/bin" "$temporary/run"
or exit 1
cp "$fake_task" "$temporary/bin/task"
or exit 1
chmod +x "$temporary/bin/task"
or exit 1
set -lx PATH "$temporary/bin" $PATH
set -gx MINOR_WRAPPER_RECEIPT "$temporary/receipt"
set -gx MINOR_WRAPPER_PREPARE_EXIT 0
set -gx MINOR_WRAPPER_RESTORE_EXIT 0
set -gx MINOR_WRAPPER_BROWSER_EXIT 0
set -g minor_test_helper "$helper"
set -g minor_test_project mtcontract-0123456789abcdef
set -g minor_test_fixture "$temporary/run/fixture.json"
printf '%s\n' "$minor_test_project" >"$temporary/run/owner"
printf '%s\n' '{"household_id":1,"view_membership_id":2,"view_account_id":3,"web_view_email":"contract-view-probe@example.test"}' >"$minor_test_fixture"

function check_wrapper -a name project files fixture expected_exit expected_phases
    set -gx MINOR_WRAPPER_EXPECT_PROJECT "$project"
    set -gx MINOR_WRAPPER_EXPECT_FILES "$files"
    set -gx MINOR_WRAPPER_EXPECT_FIXTURE "$fixture"
    if test -f "$fixture"
        set -gx MINOR_WRAPPER_EXPECT_FIXTURE (realpath -- "$fixture")
    end
    printf '' >"$MINOR_WRAPPER_RECEIPT"
    fish --no-config "$minor_test_helper" "$project" "$files" "$fixture"
    set -l actual_exit $status
    set -l phases (string join ',' -- (cat "$MINOR_WRAPPER_RECEIPT"))
    if test "$actual_exit" != "$expected_exit"; or test "$phases" != "$expected_phases"
        echo "Minor wrapper case failed: $name; exit=$actual_exit; phases=$phases" >&2
        return 1
    end
end

set -l failed 0
set -l minor tests/household-minor-readiness.test.mjs
check_wrapper normal ordinary 'tests/household-routes.test.mjs' missing 0 node; or set failed 1
check_wrapper successful "$minor_test_project" "$minor" "$minor_test_fixture" 0 prepare,node,restore; or set failed 1
set -gx MINOR_WRAPPER_BROWSER_EXIT 7
check_wrapper browser_failed "$minor_test_project" "$minor" "$minor_test_fixture" 7 prepare,node,restore; or set failed 1
set -gx MINOR_WRAPPER_BROWSER_EXIT 0
set -gx MINOR_WRAPPER_PREPARE_EXIT 8
check_wrapper prepare_failed "$minor_test_project" "$minor" "$minor_test_fixture" 8 prepare; or set failed 1
set -gx MINOR_WRAPPER_PREPARE_EXIT 0
set -gx MINOR_WRAPPER_RESTORE_EXIT 9
check_wrapper restore_failed "$minor_test_project" "$minor" "$minor_test_fixture" 9 prepare,node,restore; or set failed 1
set -gx MINOR_WRAPPER_RESTORE_EXIT 0
check_wrapper mixed "$minor_test_project" "$minor tests/household-routes.test.mjs" "$minor_test_fixture" 1 ''; or set failed 1
check_wrapper invalid_project production "$minor" "$minor_test_fixture" 1 ''; or set failed 1
check_wrapper owner_mismatch mtcontract-ffffffffffffffff "$minor" "$minor_test_fixture" 1 ''; or set failed 1
printf '%s\n' '{"household_id":1,"view_membership_id":null,"view_account_id":3,"web_view_email":"contract-view-probe@example.test"}' >"$minor_test_fixture"
check_wrapper invalid_identity "$minor_test_project" "$minor" "$minor_test_fixture" 1 ''; or set failed 1
rm -rf "$temporary"
exit $failed

