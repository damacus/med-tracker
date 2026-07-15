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

  it 'passes when migration and runtime logins have isolated role memberships' do
    stub_preflight(isolated_memberships)

    expect { task.invoke }.to output(/pre-0.5 database upgrade preflight passed/i).to_stdout
  end

  it 'aborts with the upgrade runbook when runtime roles are not bootstrapped' do
    allow(connection).to receive_messages(select_all: [], select_value: false)

    expect { task.invoke }
      .to raise_error(SystemExit)
      .and output(/pre-0.5 database upgrade/).to_stderr
  end

  it 'aborts when the migration login can assume the application role' do
    stub_preflight(cross_membership)

    expect { task.invoke }
      .to raise_error(SystemExit)
      .and output(/isolated role membership/).to_stderr
  end

  def stub_preflight(memberships)
    stub_role_rows
    stub_membership_rows(memberships)
    allow(connection).to receive(:select_value).with(/session_user/).and_return('medtracker_migration')
  end

  def stub_role_rows
    allow(connection).to receive(:select_all).with(/FROM pg_roles/).and_return(safe_role_rows)
  end

  def stub_membership_rows(memberships)
    allow(connection).to receive(:select_all).with(/FROM pg_auth_members/).and_return(memberships)
  end

  def safe_role_rows
    [
      role_row('med_tracker_app', login: false),
      role_row('med_tracker_owner', login: false),
      role_row('medtracker_auxiliary', login: true),
      role_row('medtracker_migration', login: true),
      role_row('medtracker_runtime', login: true)
    ]
  end

  def role_row(name, login:)
    {
      'rolname' => name,
      'rolcanlogin' => login,
      'rolsuper' => false,
      'rolcreaterole' => false,
      'rolcreatedb' => false,
      'rolreplication' => false,
      'rolbypassrls' => false
    }
  end

  def isolated_memberships
    [
      membership_row('med_tracker_app', 'medtracker_runtime'),
      membership_row('med_tracker_owner', 'medtracker_migration')
    ]
  end

  def cross_membership
    [membership_row('med_tracker_app', 'medtracker_migration')]
  end

  def membership_row(granted_role, member_role)
    {
      'granted_role' => granted_role,
      'member_role' => member_role,
      'inherit_option' => false,
      'set_option' => true
    }
  end
end
