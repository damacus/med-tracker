set -l project $argv[1]
set -l run_dir $argv[2]
string match -rq '^mtcontract-[a-f0-9]{16}$' -- $project
or exit 2
string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
or exit 2
test -f "$run_dir/owner"
or exit 2
test (cat "$run_dir/owner") = "$project"
or exit 2

set -lx MEDTRACKER_GIT_COMMON_DIR (rtk proxy git rev-parse --path-format=absolute --git-common-dir)
or exit $status
rtk proxy ./scripts/with_compose_lock.rb "$project-test" docker compose -p $project --profile test restart web-test
