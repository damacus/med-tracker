set -l project $argv[1]
string match -rq '^mtcontract-[a-f0-9]{16}$' -- $project
or begin; echo 'Refusing image removal for an invalid contract project' >&2; exit 2; end
set -l image medtracker-contract-runner:$project
docker image inspect $image >/dev/null 2>&1
or exit 0
docker image rm $image
