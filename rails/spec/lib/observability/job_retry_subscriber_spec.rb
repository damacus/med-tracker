# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::JobRetrySubscriber do
  let(:job) { MissedDoseNotificationJob.new(42, 43, '2026-05-12', '07:15') }
  let(:notification) do
    instance_double(
      ActiveSupport::Notifications::Event,
      payload: {
        job:,
        error: RuntimeError.new('private retry failure'),
        wait: 5
      }
    )
  end

  it 'records retry without serializing job arguments or exception text' do
    allow(Observability::NotificationStage).to receive(:emit)

    described_class.call(notification)

    expect(Observability::NotificationStage).to have_received(:emit).with(
      kind: :missed_dose,
      stage: :job_execution,
      reason: :retrying,
      error_type: RuntimeError
    )
  end
end
