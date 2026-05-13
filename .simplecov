# frozen_string_literal: true

SimpleCov.command_name ENV.fetch('SIMPLECOV_COMMAND_NAME', 'rspec')

SimpleCov.start 'rails' do
  enable_coverage :branch

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
