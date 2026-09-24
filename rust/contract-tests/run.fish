set -l mode $argv[1]
set -l fixture_path (pwd)/tmp/contract-tests/fixture.json

function remove_contract_fixture --on-event fish_exit
    rtk proxy rm -f tmp/contract-tests/fixture.json
end

if test "$mode" != rails; and test "$mode" != rust
    echo 'Expected rails or rust mode' >&2
    exit 2
end

rtk task test:server
or exit $status

rtk task --force test:exec CMD='rails runner scripts/contract_provision.rb'
or exit $status

if test "$mode" = rails
    set -l port (rtk task test:port)
    or exit $status
    rtk task contract:run BASE_URL="http://127.0.0.1:$port" FIXTURE_PATH="$fixture_path"
else
    rtk task contract:run BASE_URL="$CONTRACT_RUST_URL" FIXTURE_PATH="$fixture_path" APPROVED_ORIGIN="$CONTRACT_RUST_APPROVED_ORIGIN"
end
