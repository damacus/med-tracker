set -l mapping (rtk proxy docker compose -p $argv[1] --profile $argv[2] port $argv[3] $argv[4])
or exit $status
set -l port (string match -r --groups-only ':([0-9]+)$' -- $mapping)
or exit $status
test (count $port) -eq 1
or exit 2
echo $port
