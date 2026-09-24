set -l project $CONTRACT_PROJECT
set -l run_dir $CONTRACT_RUN_DIR

string match -rq '^mtcontract-[a-f0-9]{16}$' -- $project
or begin; echo 'Refusing cleanup for an invalid contract project' >&2; exit 2; end
string match -rq '^tmp/contract-tests/run\.[A-Za-z0-9]{6}$' -- $run_dir
or begin; echo 'Refusing cleanup outside a contract run directory' >&2; exit 2; end
test -f $run_dir/owner
or begin; echo 'Refusing cleanup without a run ownership marker' >&2; exit 2; end
test (cat $run_dir/owner | string trim) = $project
or begin; echo 'Refusing cleanup for a project not owned by this run' >&2; exit 2; end
test (realpath (dirname (realpath $run_dir))) = (realpath tmp/contract-tests)
or begin; echo 'Refusing cleanup outside this worktree' >&2; exit 2; end
