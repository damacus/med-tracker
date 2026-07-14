# frozen_string_literal: true

require 'rails_helper'
require 'rake'
require 'tmpdir'

Rails.application.load_tasks unless Rake::Task.task_defined?('household_lifecycle:export')

RSpec.describe HouseholdLifecycleTasks do
  around do |example|
    previous = ENV.to_h.slice(
      'HOUSEHOLD_ID', 'ACTOR_ACCOUNT_ID', 'MEMBERSHIP_ID', 'EXPORT_ID', 'DESTINATION',
      'HOUSEHOLD_EXPORT_OUTPUT_ROOT', 'REASON', 'REVIEW_ON', 'HOLD_ID'
    )
    example.run
  ensure
    %w[
      HOUSEHOLD_ID ACTOR_ACCOUNT_ID MEMBERSHIP_ID EXPORT_ID DESTINATION HOUSEHOLD_EXPORT_OUTPUT_ROOT
      REASON REVIEW_ON HOLD_ID
    ].each { |key| ENV.delete(key) }
    previous.each { |key, value| ENV[key] = value }
  end

  it 'prints sanitized export evidence' do
    household = instance_double(Household, id: 41)
    account = instance_double(Account, id: 42)
    membership = instance_double(HouseholdMembership, id: 43)
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

  it 'boots a household export through the forced-RLS application role' do
    evidence, household = forced_rls_export_evidence

    expect(evidence).to include(
      'event_type' => 'household.export.ready',
      'outcome' => 'ready',
      'household_id' => household.id
    )
  end

  it 'downloads an authorized export to a protected operator destination under forced RLS' do
    actor, household, membership = export_task_context('forced-rls-export-download-task@example.test')
    export = Households::HostedExport.generate!(household: household, membership: membership, actor_account: actor)

    Dir.mktmpdir('hosted-export-output') do |output_root|
      result = download_task_result(actor, household, export, output_root)

      expect(result.fetch(:evidence)).to include(expected_download_evidence(household, export, result.fetch(:bytes)))
      expect(result.fetch(:evidence).to_json).not_to include(result.fetch(:destination))
      verify_download_artifact(result)
    end
  end

  def forced_rls_export_evidence
    actor, household, membership = export_task_context('forced-rls-export-task@example.test')
    ENV.update(
      'HOUSEHOLD_ID' => household.id.to_s,
      'ACTOR_ACCOUNT_ID' => actor.id.to_s,
      'MEMBERSHIP_ID' => membership.id.to_s
    )
    output = with_runtime_role { capture_stdout { invoke('household_lifecycle:export') } }
    [JSON.parse(output), household]
  end

  def export_task_context(email)
    actor = Account.create!(email: email, status: :verified)
    household = create(:household)
    membership = household.household_memberships.create!(
      account: actor,
      role: :owner,
      status: :active,
      joined_at: Time.current
    )
    [actor, household, membership]
  end

  def download_task_result(actor, household, export, output_root)
    bytes = export.artifact.download
    destination, evidence = download_task_evidence(actor, household, export, output_root)
    { bytes: bytes, destination: destination, evidence: evidence }
  end

  def expected_download_evidence(household, export, bytes)
    {
      'event_type' => 'household.export.downloaded', 'outcome' => 'downloaded',
      'household_id' => household.id, 'export_id' => export.id,
      'artifact_byte_size' => bytes.bytesize,
      'artifact_checksum_sha256' => Digest::SHA256.hexdigest(bytes)
    }
  end

  def verify_download_artifact(result)
    destination = result.fetch(:destination)
    expect(File.binread(destination)).to eq(result.fetch(:bytes))
    expect(File.stat(destination).mode & 0o777).to eq(0o600)
  end

  def download_task_evidence(actor, household, export, output_root)
    destination = File.join(output_root, 'household-export.zip')
    configure_download_environment(household, actor, export, output_root, destination)
    output = with_runtime_role { capture_stdout { invoke('household_lifecycle:download') } }
    [destination, JSON.parse(output)]
  end

  def configure_download_environment(household, actor, export, output_root, destination)
    ENV.update(
      'HOUSEHOLD_ID' => household.id.to_s,
      'ACTOR_ACCOUNT_ID' => actor.id.to_s,
      'EXPORT_ID' => export.id.to_s,
      'HOUSEHOLD_EXPORT_OUTPUT_ROOT' => output_root,
      'DESTINATION' => destination
    )
  end

  it 'passes hostile hold reason text unchanged and prints identifiers without the reason' do
    stub_hold_task
    reason = 'Preserve "urgent"; $(touch /tmp/medtracker-hold-injection) & review'
    configure_hold_environment(reason)

    expected = JSON.generate(event_type: 'household.retention_hold.placed', outcome: 'active', household_id: 41,
                             retention_hold_id: 61, review_on: '2026-08-13')
    expect { invoke('household_lifecycle:hold') }.to output("#{expected}\n").to_stdout
    expect(Households::RetentionHoldManager).to have_received(:place!).with(
      household: anything, actor_account: anything, reason: reason,
      review_on: Date.new(2026, 8, 13)
    )
    expect(expected).not_to include(reason)
  end

  def configure_hold_environment(reason)
    ENV.update(
      'HOUSEHOLD_ID' => '41',
      'ACTOR_ACCOUNT_ID' => '42',
      'REASON' => reason,
      'REVIEW_ON' => '2026-08-13'
    )
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

  def with_runtime_role
    result = nil
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      result = yield
      raise ActiveRecord::Rollback
    end
    result
  end

  def capture_stdout
    original = $stdout
    $stdout = StringIO.new
    yield
    $stdout.string
  ensure
    $stdout = original
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
