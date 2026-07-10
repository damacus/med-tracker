# frozen_string_literal: true

SimpleCov.configure do
  load_profile 'rails'

  command_name ENV.fetch('SIMPLECOV_COMMAND_NAME', 'rspec')
  enable_coverage :branch

  # Enforced ratchet (fails the build below threshold). Measured non-system
  # baseline 2026-06-09 after the coverage+mutation pass: line 90.3% /
  # branch 75.24%, rounded down. Raise these as coverage improves; never lower
  # without a recorded reason. Only enforced when COVERAGE=true (CI's non-system job).
  minimum_coverage line: 90, branch: 75

  add_filter '/spec/'
  add_filter '/config/'
  add_filter '/vendor/'
  add_filter '/tmp/'

  add_group 'API' do |source_file|
    filename = source_file.filename

    filename.include?('/app/controllers/api/') ||
      filename.match?(%r{/app/models/api_.*\.rb\z}) ||
      filename.include?('/app/serializers/api/') ||
      filename.include?('/app/serializers/fhir/') ||
      filename.include?('/app/services/api/')
  end
  add_group 'Components', 'app/components'
  add_group 'Controllers', 'app/controllers'
  add_group 'Domain', 'app/domain'
  add_group 'Jobs', 'app/jobs'
  add_group 'Mailers', 'app/mailers'
  add_group 'Models', 'app/models'
  add_group 'Policies', 'app/policies'
  add_group 'Presenters', 'app/presenters'
  add_group 'Serializers', 'app/serializers'
  add_group 'Services', 'app/services'

  at_exit do
    result = SimpleCov.result
    next unless result

    result.format!
    next unless ENV['COVERAGE'] == 'true'

    api_group = result.groups.fetch('API')
    api_total = api_group.total_branches.to_i
    api_covered = api_group.covered_branches.to_i
    api_percent = api_total.zero? ? 100.0 : (api_covered.to_f / api_total * 100)
    next if api_percent >= 90.0

    warn format('API branch coverage (%.2f%%) is below the configured minimum coverage (90.00%%).',
                api_percent)
    Kernel.exit SimpleCov::ExitCodes::MINIMUM_COVERAGE
  end
end
