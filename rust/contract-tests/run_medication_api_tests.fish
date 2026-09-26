set -l targets --test medication_read_api --test medication_mobile_oauth_api --test medication_forecast_api --test oauth --test dose_write_api --test web_session_api --test web_reads_api --test source_capabilities_api --test medication_stock --test medication_management_security_api --test dose_mode_transition_api --test management_sync_events_api
if test "$argv[1]" = --no-run
    cargo test --locked --manifest-path rust/contract-tests/Cargo.toml $targets --no-run
else
    cargo test --locked --no-fail-fast --manifest-path rust/contract-tests/Cargo.toml $targets -- --test-threads=1
end
