# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task_name) { 'med_tracker:storage:verify_restore' }
  let(:task) { described_class[task_name] }

  before do
    Rails.application.load_tasks unless described_class.task_defined?(task_name)
    task.reenable
  end

  around do |example|
    previous = ENV.to_h.slice(
      'ATTACHMENT_ID',
      'DATABASE_RECOVERY_REFERENCE',
      'DISK_RECOVERY_REFERENCE',
      'S3_RECOVERY_REFERENCE',
      'AUTHORIZED_RETRIEVAL_VERIFIED',
      'CROSS_HOUSEHOLD_DENIAL_VERIFIED',
      'STORAGE_MIGRATION_RUN_ID'
    )
    ENV.update(
      'DATABASE_RECOVERY_REFERENCE' => 'database-snapshot-1',
      'DISK_RECOVERY_REFERENCE' => 'disk-snapshot-1',
      'AUTHORIZED_RETRIEVAL_VERIFIED' => 'true',
      'CROSS_HOUSEHOLD_DENIAL_VERIFIED' => 'true'
    )
    example.run
  ensure
    %w[
      ATTACHMENT_ID DATABASE_RECOVERY_REFERENCE DISK_RECOVERY_REFERENCE S3_RECOVERY_REFERENCE
      AUTHORIZED_RETRIEVAL_VERIFIED CROSS_HOUSEHOLD_DENIAL_VERIFIED STORAGE_MIGRATION_RUN_ID
    ].each { ENV.delete(it) }
    ENV.update(previous)
  end

  it 'reports only the database and required storage recovery references without health data' do
    configure_successful_restore
    output = capture_stdout { task.invoke }

    expect(JSON.parse(output)).to include(expected_recovery_payload)
    expect(output).not_to include('unused-s3-snapshot')
  end

  def configure_successful_restore
    ENV['ATTACHMENT_ID'] = '42'
    ENV['S3_RECOVERY_REFERENCE'] = 'unused-s3-snapshot'
    result = Storage::RestoreVerifier::Result.new(
      attachment_id: 42,
      blob_id: 84,
      byte_size: 512,
      required_backends: [:disk],
      authorized_retrieval: true,
      cross_household_denied: true
    )
    allow(Storage::RestoreVerifier).to receive(:call).and_return(result)
    allow(ActiveRecord::Base.connection).to receive(:select_value)
      .with('SELECT current_user').and_return('med_tracker_owner')
  end

  def expected_recovery_payload
    {
      'outcome' => 'passed',
      'attachment_id' => 42,
      'required_backends' => ['disk'],
      'database_reference' => 'database-snapshot-1',
      'storage_references' => { 'disk' => 'disk-snapshot-1' }
    }
  end

  it 'fails the smoke check when verification fails' do
    ENV['ATTACHMENT_ID'] = nil
    allow(ActiveRecord::Base.connection).to receive(:select_value)
      .with('SELECT current_user').and_return('med_tracker_owner')
    allow(Storage::RestoreVerifier).to receive(:call)
      .and_raise(Storage::RestoreVerifier::VerificationError, 'No attachment found')

    expect { task.invoke }.to raise_error(SystemExit)
      .and output(/"outcome":"failed","failure_code":"restore_verification_failed"/).to_stderr
  end

  it 'rejects a non-owner database role before selecting restored records' do
    allow(ActiveRecord::Base.connection).to receive(:select_value)
      .with('SELECT current_user').and_return('med_tracker_app')
    allow(Storage::RestoreVerifier).to receive(:call)

    expect { task.invoke }.to raise_error(SystemExit)
      .and output(/"failure_code":"owner_role_required"/).to_stderr
    expect(Storage::RestoreVerifier).not_to have_received(:call)
  end

  def capture_stdout
    original = $stdout
    stream = StringIO.new
    $stdout = stream
    yield
    stream.string
  ensure
    $stdout = original
  end
end
