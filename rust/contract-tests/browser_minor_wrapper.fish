function main -a project browser_files fixture_path
    set -l minor_file tests/household-minor-readiness.test.mjs
    set -l files (string split -n ' ' -- "$browser_files")
    if not contains -- $minor_file $files
        rtk proxy task api:contract-browser-node "CONTRACT_PROJECT=$project" "BROWSER_TEST_FILES=$browser_files"
        return $status
    end
    if test (count $files) -ne 1
        echo 'Minor browser checks require their own disposable fixture' >&2
        return 1
    end
    if not string match -qr '^mtcontract-[0-9a-f]{16}$' -- "$project"
        echo 'Minor browser checks require a disposable contract project' >&2
        return 1
    end
    if not test -f "$fixture_path"
        echo 'Minor browser fixture is missing' >&2
        return 1
    end
    set -l actual_fixture (realpath -- "$fixture_path")
    or return 1
    if test (path basename "$actual_fixture") != fixture.json
        echo 'Minor browser fixture has an unexpected filename' >&2
        return 1
    end
    set -l owner_file (path dirname "$actual_fixture")/owner
    if not test -f "$owner_file"
        echo 'Minor browser fixture has no ownership marker' >&2
        return 1
    end
    set -l fixture_owner (string trim -- (cat -- "$owner_file"))
    if test "$fixture_owner" != "$project"
        echo 'Minor browser fixture does not belong to this project' >&2
        return 1
    end
    jq -e '
      def positive_id: type == "number" and . > 0 and . == floor;
      (.household_id | positive_id) and
      (.view_membership_id | positive_id) and
      (.view_account_id | positive_id) and
      (.web_view_email | type == "string" and test("^contract-view-[^[:space:]@]+@example[.]test$"))
    ' "$actual_fixture" >/dev/null
    if test $status -ne 0
        echo 'Minor browser fixture identities are invalid' >&2
        return 1
    end
    rtk proxy task api:contract-minor-viewer-sql "CONTRACT_PROJECT=$project" PHASE=prepare "FIXTURE_PATH=$actual_fixture"
    set -l prepare_exit $status
    if test $prepare_exit -ne 0
        return $prepare_exit
    end
    rtk proxy task api:contract-browser-node "CONTRACT_PROJECT=$project" "BROWSER_TEST_FILES=$browser_files"
    set -l browser_exit $status
    rtk proxy task api:contract-minor-viewer-sql "CONTRACT_PROJECT=$project" PHASE=restore "FIXTURE_PATH=$actual_fixture"
    set -l restore_exit $status
    if test $restore_exit -ne 0
        echo 'Minor browser fixture restoration failed' >&2
        return $restore_exit
    end
    return $browser_exit
end

main $argv
