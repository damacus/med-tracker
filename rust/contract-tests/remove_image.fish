set -l project $argv[1]
string match -rq '^mtcontract-[a-f0-9]{16}$' -- $project
or begin; echo 'Refusing image removal for an invalid contract project' >&2; exit 2; end
for image in medtracker-contract-runner:$project medtracker-contract-browser:$project
    if docker image inspect $image >/dev/null 2>&1
        docker image rm $image
        or exit $status
    end
end
