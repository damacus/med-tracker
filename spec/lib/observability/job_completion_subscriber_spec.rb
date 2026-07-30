# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::JobCompletionSubscriber do
  let(:job) do
    instance_double(
      ActiveJob::Base,
      class: ScheduleDailyRemindersJob,
      job_id: '4a42ed43-4cf2-496e-a180-6829ce4efc06',
      queue_name: 'default',
      arguments: ['household=41', 'schedule=52']
    )
  end
  let(:notification) do
    instance_double(
      ActiveSupport::Notifications::Event,
      duration: 14.5,
      payload: { job: }
    )
  end

  before { allow(Observability::Publisher).to receive(:publish) }

  it 'publishes one canonical job completion' do
    described_class.call(notification)

    expect(Observability::Publisher).to have_received(:publish).once do |event|
      expect(event.to_h).to include(
        'event.name' => 'job.completed',
        'event.dataset' => 'medtracker.job'
      )
      expect(event.to_json).not_to include('household=41', 'schedule=52')
    end
  end

  it 'records a safe notification job failure outcome' do
    allow(job).to receive(:class).and_return(MissedDoseNotificationJob)
    notification.payload[:exception_object] = RuntimeError.new('private job failure')
    allow(Observability::NotificationStage).to receive(:emit)

    described_class.call(notification)

    expect(Observability::NotificationStage).to have_received(:emit).with(
      kind: :missed_dose,
      stage: :job_execution,
      reason: :job_failed,
      error_type: RuntimeError
    )
  end
end
