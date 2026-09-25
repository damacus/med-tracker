set -l root (pwd)
set -l first (string lower (string replace -a - '' (uuidgen)) | string sub -l 16)
set -l second (string lower (string replace -a - '' (uuidgen)) | string sub -l 16)
set -g first_project mtcontract-$first
set -g second_project mtcontract-$second
rtk proxy mkdir -p tmp/contract-tests
or exit $status
set -g first_dir (rtk proxy mktemp -d tmp/contract-tests/run.XXXXXX)
set -g second_dir (rtk proxy mktemp -d tmp/contract-tests/run.XXXXXX)
set -l first_fixture $root/$first_dir/fixture.json
set -l second_fixture $root/$second_dir/fixture.json
set -lx CONTRACT_DATABASE_URL postgresql://medtracker:medtracker_password@db-test:5432/medtracker_contract
set -lx CONTRACT_RATE_LIMITING true
set -g first_subnet $CONTRACT_ISOLATION_FIRST_SUBNET
set -g second_subnet $CONTRACT_ISOLATION_SECOND_SUBNET
if test -n "$first_subnet"; or test -n "$second_subnet"
    test -n "$first_subnet"; and test -n "$second_subnet"; and test "$first_subnet" != "$second_subnet"
    or begin; echo 'Isolation subnets must be two distinct /28 networks' >&2; exit 2; end
    rtk task api:contract-subnet-check CONTRACT_TEST_SUBNET=$first_subnet
    or exit $status
    rtk task api:contract-subnet-check CONTRACT_TEST_SUBNET=$second_subnet
    or exit $status
end

function isolation_task -a project
    set -l subnet
    if test -n "$first_subnet"
        if test "$project" = "$first_project"
            set subnet $first_subnet
        else if test "$project" = "$second_project"
            set subnet $second_subnet
        else
            echo "Unknown isolation project: $project" >&2
            return 2
        end
    end
    if test -n "$subnet"
        set -lx CONTRACT_TEST_SUBNET $subnet
        set -lx COMPOSE_FILE compose.yaml:rust/contract-tests/runner-subnet.compose.yaml
        rtk task $argv[2..-1]
    else
        rtk task $argv[2..-1]
    end
end

function cleanup_isolation_test --on-event fish_exit
    for pair in "$first_project:$first_dir" "$second_project:$second_dir"
        set -l parts (string split : $pair)
        if test -f $parts[2]/owner
            isolation_task $parts[1] contract:cleanup CONTRACT_PROJECT=$parts[1] CONTRACT_RUN_DIR=$parts[2]
            or begin
                echo "Cleanup failed; ownership marker retained at $parts[2]/owner" >&2
                continue
            end
        end
        rtk proxy rm -f $parts[2]/fixture.json $parts[2]/owner
        rtk proxy rmdir $parts[2]
    end
    if set -q failure_log
        rtk proxy rm -f $failure_log
    end
end

echo $first_project > $first_dir/owner
echo $second_project > $second_dir/owner

for pair in "$first_project:$first_dir" "$second_project:$second_dir"
    set -l parts (string split : $pair)
    isolation_task $parts[1] contract:prepare-db CONTRACT_PROJECT=$parts[1]
    or exit $status
    isolation_task $parts[1] test:server CONTRACT_PROJECT=$parts[1]
    or exit $status
    isolation_task $parts[1] --force test:exec CONTRACT_PROJECT=$parts[1] CMD="CONTRACT_FIXTURE_PATH=/app/$parts[2]/fixture.json rails runner scripts/contract_provision.rb"
    or exit $status
end

set -l first_email (rtk proxy jq -r .primary_email $first_fixture)
set -l second_email (rtk proxy jq -r .primary_email $second_fixture)
set -l first_port (isolation_task $first_project test:port CONTRACT_PROJECT=$first_project)
set -l second_port (isolation_task $second_project test:port CONTRACT_PROJECT=$second_project)
test "$first_port" != "$second_port"
or begin; echo 'Runs share a web server port' >&2; exit 1; end

for entry in "$first_project:$first_email:$second_email" "$second_project:$second_email:$first_email"
    set -l fields (string split : $entry)
    set -l counts (isolation_task $fields[1] --force test:exec CONTRACT_PROJECT=$fields[1] CMD="rails runner \"puts [Account.where(email: '$fields[2]').count, Account.where(email: '$fields[3]').count, ActiveRecord::Base.connection.select_value('SHOW server_version')].join(':')\"" | string match -r '^[0-9]+:[0-9]+:[0-9].*' | tail -1)
    string match -rq '^1:0:18\.' -- $counts
    or begin; echo "Fixture isolation or PostgreSQL version failed for $fields[1]: $counts" >&2; exit 1; end
    echo "$fields[1] accounts and PostgreSQL version: $counts"
end

isolation_task $first_project contract:cleanup CONTRACT_PROJECT=$first_project CONTRACT_RUN_DIR=$first_dir
or exit $status
rtk proxy rm -f $first_dir/owner
test (rtk proxy docker ps -aq --filter label=com.docker.compose.project=$first_project | count) -eq 0
or begin; echo 'First run containers survived cleanup' >&2; exit 1; end
test (rtk proxy docker volume ls -q --filter label=com.docker.compose.project=$first_project | count) -eq 0
or begin; echo 'First run volumes survived cleanup' >&2; exit 1; end
test (rtk proxy docker image ls -q --filter reference=$first_project-web-test | count) -eq 0
or begin; echo 'First run web image survived cleanup' >&2; exit 1; end
set -l remaining (isolation_task $second_project --force test:exec CONTRACT_PROJECT=$second_project CMD="rails runner \"puts Account.where(email: '$second_email').count\"" | string match -r '^[0-9]+$' | tail -1)
test "$remaining" = 1
or begin; echo 'Peer fixture disappeared after first run cleanup' >&2; exit 1; end

set -g failure_log (rtk proxy mktemp tmp/contract-tests/failure.XXXXXX)
set -lx CONTRACT_RUST_URL http://127.0.0.1:1
set -l failure_status
begin
    if test -n "$first_subnet"
        set -lx CONTRACT_TEST_SUBNET $first_subnet
        rtk proxy fish rust/contract-tests/run.fish rust sync > $failure_log 2>&1
        set failure_status $status
    else
        rtk proxy fish rust/contract-tests/run.fish rust sync > $failure_log 2>&1
        set failure_status $status
    end
end
test $failure_status -ne 0
or begin; echo 'Expected Rust target failure did not occur' >&2; exit 1; end
set -l failed_project (string match -r -m1 'mtcontract-[a-f0-9]{16}' < $failure_log)
test -n "$failed_project"
or begin; echo 'Failure run did not report its project' >&2; exit 1; end
test (rtk proxy docker ps -aq --filter label=com.docker.compose.project=$failed_project | count) -eq 0
or begin; echo 'Failure run containers survived cleanup' >&2; exit 1; end
test (rtk proxy docker volume ls -q --filter label=com.docker.compose.project=$failed_project | count) -eq 0
or begin; echo 'Failure run volumes survived cleanup' >&2; exit 1; end
test (rtk proxy docker image ls -q --filter reference=$failed_project-web-test | count) -eq 0
or begin; echo 'Failure run web image survived cleanup' >&2; exit 1; end
set -l after_failure (isolation_task $second_project --force test:exec CONTRACT_PROJECT=$second_project CMD="rails runner \"puts Account.where(email: '$second_email').count\"" | string match -r '^[0-9]+$' | tail -1)
test "$after_failure" = 1
or begin; echo 'Peer fixture disappeared after failure cleanup' >&2; exit 1; end

isolation_task $second_project contract:cleanup CONTRACT_PROJECT=$second_project CONTRACT_RUN_DIR=$second_dir
or exit $status
rtk proxy rm -f $second_dir/owner
test (rtk proxy docker ps -aq --filter label=com.docker.compose.project=$second_project | count) -eq 0
or begin; echo 'Second run containers survived cleanup' >&2; exit 1; end
test (rtk proxy docker volume ls -q --filter label=com.docker.compose.project=$second_project | count) -eq 0
or begin; echo 'Second run volumes survived cleanup' >&2; exit 1; end
test (rtk proxy docker image ls -q --filter reference=$second_project-web-test | count) -eq 0
or begin; echo 'Second run web image survived cleanup' >&2; exit 1; end

echo 'Concurrent fixture isolation and success/failure peer-safe cleanup passed'
