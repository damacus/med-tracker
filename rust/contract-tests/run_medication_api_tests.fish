set -l targets --test medication_read_api --test medication_mobile_oauth_api --test medication_forecast_api --test oauth
if test "$argv[1]" = --no-run
    cargo test --locked --manifest-path rust/contract-tests/Cargo.toml $targets --no-run
else
    cargo test --locked --manifest-path rust/contract-tests/Cargo.toml $targets -- --test-threads=1
end
