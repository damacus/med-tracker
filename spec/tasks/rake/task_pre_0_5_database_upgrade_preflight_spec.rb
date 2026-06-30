# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task_name) { 'med_tracker:pre_0_5_database_upgrade_preflight' }
  let(:task) { described_class[task_name] }
  let(:connection) { instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter) }

  before do
    Rails.application.load_tasks unless described_class.task_defined?(task_name)
    task.reenable
    allow(ActiveRecord::Base).to receive(:connection).and_return(connection)
  end

  it 'passes when runtime roles are bootstrapped for the app login' do
    allow(connection).to receive(:select_all).and_return(
      [
        { 'rolname' => 'med_tracker_app', 'rolcanlogin' => false, 'rolsuper' => false, 'rolbypassrls' => false },
        { 'rolname' => 'med_tracker_owner', 'rolcanlogin' => false, 'rolsuper' => false, 'rolbypassrls' => false }
      ]
    )
    allow(connection).to receive(:quote) { |value| "'#{value}'" }
    allow(connection).to receive(:select_value).with(/med_tracker_owner/).and_return(true)
    allow(connection).to receive(:select_value).with(/med_tracker_app/).and_return(true)

    expect { task.invoke }.to output(/pre-0.5 database upgrade preflight passed/i).to_stdout
  end

  it 'aborts with the upgrade runbook when runtime roles are not bootstrapped' do
    allow(connection).to receive_messages(select_all: [], select_value: false)

    expect { task.invoke }
      .to raise_error(SystemExit)
      .and output(/pre-0.5 database upgrade/).to_stderr
  end
end
