if test (string join ' ' -- $argv) != 'rust/contract-tests/run.fish rails browser-journey-rust'
    echo 'Unexpected household wrapper command' >&2
    exit 1
end

if not set -q SELECTOR_PROBE_RECEIPT
    echo 'Missing household wrapper receipt path' >&2
    exit 1
end

printf '%s\n' "$HOUSEHOLD_ACCEPTANCE" "$HOUSEHOLD_COMPLETION_ACCEPTANCE" "$HOUSEHOLD_STOCK_ACCEPTANCE" "$HOUSEHOLD_TEST_FILE" "$HOUSEHOLD_TEST_FILTER" "$BROWSER_TEST_FILES" >$SELECTOR_PROBE_RECEIPT
