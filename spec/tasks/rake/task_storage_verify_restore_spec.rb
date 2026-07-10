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
    previous_attachment_id = ENV.fetch('ATTACHMENT_ID', nil)
    example.run
  ensure
    ENV['ATTACHMENT_ID'] = previous_attachment_id
  end

  it 'reports the restored attachment and blob without health data' do
    ENV['ATTACHMENT_ID'] = '42'
    result = Storage::RestoreVerifier::Result.new(attachment_id: 42, blob_id: 84, byte_size: 512)
    allow(Storage::RestoreVerifier).to receive(:call).with(attachment_id: '42').and_return(result)

    expect { task.invoke }
      .to output("Storage restore verified: attachment_id=42 blob_id=84 byte_size=512\n").to_stdout
  end

  it 'fails the smoke check when verification fails' do
    ENV['ATTACHMENT_ID'] = nil
    allow(Storage::RestoreVerifier).to receive(:call)
      .with(attachment_id: nil)
      .and_raise(Storage::RestoreVerifier::VerificationError, 'No attachment found')

    expect { task.invoke }.to raise_error(SystemExit)
      .and output(/Storage restore verification failed: No attachment found/).to_stderr
  end
end
