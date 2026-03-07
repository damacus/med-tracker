1. Update GitHub Actions Workflows
   - In `.github/workflows/ci.yml`, `ruby/setup-ruby@v1` has been replaced with `appraisal-rb/setup-ruby-flash@v1`. This uses `rv` and `ore` to speed up Ruby and gem installation.
2. Verify changes to GitHub Actions Workflows
   - Use `read_file` to confirm the edits to `.github/workflows/ci.yml` were written correctly.
3. Update `Dockerfile` to use `ore`
   - Modify the `Dockerfile` to install `ore-light` system-wide.
   - Replace `RUN bundle install` with `RUN ore install` for faster gem installation.
   - Add the curl script `curl -fsSL https://raw.githubusercontent.com/contriboss/ore-light/master/scripts/install.sh | bash -s -- --system` to the package installation step in the `Dockerfile`.
4. Verify `Dockerfile` edit
   - Use `read_file` to confirm the edits to the `Dockerfile` were written correctly.
5. Update scripts in `bin/` directory
   - Update `bin/setup` to use `ore check` and `ore install` instead of `bundle check` and `bundle install`.
   - Update `bin/docker-entrypoint-web` to use `ore check` and `ore install` instead of `bundle check` and `bundle install`.
6. Verify `bin/` directory scripts edit
   - Use `read_file` to confirm the edits to `bin/setup` and `bin/docker-entrypoint-web` were written correctly.
7. Verify tests pass
   - Run tests and verifications using `task test` to ensure the application starts and tests pass.
