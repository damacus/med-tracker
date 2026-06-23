# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task_name) { 'households:migrate_local' }
  let(:task) { described_class[task_name] }

  before do
    Rails.application.load_tasks unless described_class.task_defined?(task_name)
    task.reenable
  end

  around do |example|
    previous_values = {
      'OWNER_EMAIL' => ENV.fetch('OWNER_EMAIL', nil),
      'HOUSEHOLD_NAME' => ENV.fetch('HOUSEHOLD_NAME', nil),
      'DRY_RUN' => ENV.fetch('DRY_RUN', nil),
      'APPLY' => ENV.fetch('APPLY', nil)
    }

    example.run
  ensure
    previous_values.each { |key, value| ENV[key] = value }
  end

  it 'invokes the local migrator in dry-run mode' do
    ENV['OWNER_EMAIL'] = 'owner@example.test'
    ENV['HOUSEHOLD_NAME'] = 'Owner Household'
    ENV['DRY_RUN'] = '1'
    ENV.delete('APPLY')
    result = instance_double(Households::LocalMigrator::Result, applied?: false, summary_lines: ['dry run'])

    allow(Households::LocalMigrator).to receive(:new).and_return(instance_double(Households::LocalMigrator,
                                                                                 call: result))

    expect { task.invoke }.to output(/dry run/).to_stdout
    expect(Households::LocalMigrator).to have_received(:new).with(
      owner_email: 'owner@example.test',
      household_name: 'Owner Household',
      apply: false
    )
  end
end
