# frozen_string_literal: true

require 'rails_helper'
require 'rake'

Rails.application.load_tasks unless Rake::Task.task_defined?('household_lifecycle:export')

RSpec.describe HouseholdLifecycleTasks do
  around do |example|
    previous = ENV.to_h.slice(
      'HOUSEHOLD_ID', 'ACTOR_ACCOUNT_ID', 'MEMBERSHIP_ID', 'REASON', 'REVIEW_ON', 'HOLD_ID'
    )
    example.run
  ensure
    %w[HOUSEHOLD_ID ACTOR_ACCOUNT_ID MEMBERSHIP_ID REASON REVIEW_ON HOLD_ID].each { |key| ENV.delete(key) }
    previous.each { |key, value| ENV[key] = value }
  end

  it 'prints sanitized export evidence' do
    household = instance_double(Household, id: 41)
    account = instance_double(Account)
    membership = instance_double(HouseholdMembership)
    export = instance_double(HouseholdExport, id: 51, status: 'ready', manifest: { 'attachments' => [{}, {}] })
    allow(Household).to receive(:find).with('41').and_return(household)
    allow(Account).to receive(:find).with('42').and_return(account)
    allow(HouseholdMembership).to receive(:find).with('43').and_return(membership)
    allow(Households::HostedExport).to receive(:generate!).and_return(export)
    ENV.update('HOUSEHOLD_ID' => '41', 'ACTOR_ACCOUNT_ID' => '42', 'MEMBERSHIP_ID' => '43')

    expected = JSON.generate(event_type: 'household.export.ready', outcome: 'ready', household_id: 41,
                             export_id: 51, attachment_count: 2)
    expect { invoke('household_lifecycle:export') }.to output("#{expected}\n").to_stdout
  end

  it 'passes hostile hold reason text unchanged and prints identifiers without the reason' do
    stub_hold_task
    reason = 'Preserve "urgent"; $(touch /tmp/medtracker-hold-injection) & review'
    ENV.update(
      'HOUSEHOLD_ID' => '41',
      'ACTOR_ACCOUNT_ID' => '42',
      'REASON' => reason,
      'REVIEW_ON' => '2026-08-13'
    )

    expected = JSON.generate(event_type: 'household.retention_hold.placed', outcome: 'active', household_id: 41,
                             retention_hold_id: 61, review_on: '2026-08-13')
    expect { invoke('household_lifecycle:hold') }.to output("#{expected}\n").to_stdout
    expect(Households::RetentionHoldManager).to have_received(:place!).with(
      household: anything, actor_account: anything, reason: reason,
      review_on: Date.new(2026, 8, 13)
    )
    expect(expected).not_to include(reason)
  end

  it 'prints resumable purge evidence' do
    household = instance_double(Household, id: 41)
    account = instance_double(Account)
    run = instance_double(HouseholdPurgeRun, id: 71, status: 'completed', attempts: 2,
                                             last_completed_table: 'people')
    allow(Household).to receive(:find).and_return(household)
    allow(Account).to receive(:find).and_return(account)
    allow(Households::Purger).to receive(:call).and_return(run)
    ENV.update('HOUSEHOLD_ID' => '41', 'ACTOR_ACCOUNT_ID' => '42')

    expected = JSON.generate(event_type: 'household.purge.completed', outcome: 'completed', household_id: 41,
                             purge_run_id: 71, attempts: 2, last_completed_table: 'people')
    expect { invoke('household_lifecycle:purge') }.to output("#{expected}\n").to_stdout
  end

  it 'raises a sanitized failure without retaining a sensitive exception cause' do
    allow(Household).to receive(:find).and_raise(StandardError, 'sensitive household detail')
    ENV.update('HOUSEHOLD_ID' => '41', 'ACTOR_ACCOUNT_ID' => '42')

    failure = raise_error(
      HouseholdLifecycleTasks::Failure, 'household.offboarded failed (StandardError)'
    ) do |error|
      expect(error.cause).to be_nil
    end
    expectation = output(
      "{\"event_type\":\"household.offboarded\",\"outcome\":\"failure\",\"failure_code\":\"StandardError\"}\n"
    ).to_stderr.and(failure)

    expect { invoke('household_lifecycle:offboard') }.to expectation
  end

  def invoke(task_name)
    task = Rake::Task[task_name]
    task.reenable
    task.invoke
  end

  def stub_hold_task
    household = instance_double(Household, id: 41)
    account = instance_double(Account)
    hold = instance_double(HouseholdRetentionHold, id: 61, status: 'active', review_on: Date.new(2026, 8, 13))
    allow(Household).to receive(:find).and_return(household)
    allow(Account).to receive(:find).and_return(account)
    allow(Households::RetentionHoldManager).to receive(:place!).and_return(hold)
  end
end
