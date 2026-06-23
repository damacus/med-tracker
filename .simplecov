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
end
