# frozen_string_literal: true

require 'rails_helper'
require 'yaml'

module NonBrowserCi
end

RSpec.describe NonBrowserCi do
  let(:workflow) { YAML.load_file(Rails.root.join('.github/workflows/ci.yml')) }
  let(:job) { workflow.fetch('jobs').fetch('test_non_system') }

  it 'runs every non-browser example without a timing manifest' do
    commands = job.fetch('steps').filter_map { |step| step['run'] }

    expect(commands.grep(/rspec/)).to eq(['bundle exec rspec --format RSpec::Github::Formatter --tag ~browser'])
    expect(job).not_to have_key('strategy')
  end

  it 'enforces coverage in the test process' do
    expect(job.fetch('env').fetch('COVERAGE')).to be(true)
    expect(job.fetch('env')).not_to have_key('SIMPLECOV_SHARD')
    expect(job).not_to have_key('continue-on-error')
  end

  it 'gates the test result without a separate coverage job' do
    dependencies = workflow.fetch('jobs').fetch('ci_success').fetch('needs')

    expect(dependencies).to include('test_non_system')
    expect(dependencies).not_to include('coverage')
    expect(workflow.fetch('jobs')).not_to have_key('coverage')
  end
end
