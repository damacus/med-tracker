source rust/contract-tests/cleanup.fish
or exit $status
test -d "$CONTRACT_RUN_DIR/storage"; and not test -L "$CONTRACT_RUN_DIR/storage"
or begin; echo 'Refusing ownership repair without real run storage' >&2; exit 2; end
set -l storage (realpath "$CONTRACT_RUN_DIR/storage")
test (dirname "$storage") = (realpath "$CONTRACT_RUN_DIR")
or begin; echo 'Refusing ownership repair outside the owned run directory' >&2; exit 2; end
rtk proxy docker run --rm --network none --read-only --cap-drop ALL --cap-add CHOWN --cap-add DAC_OVERRIDE --user 0 --mount "type=bind,source=$storage,target=/storage" medtracker-contract-runner:$CONTRACT_PROJECT chown -R --no-dereference (id -u):(id -g) /storage
