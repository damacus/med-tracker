# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::JobEvent do
  let(:completed_event) do
    described_class.from(
      job_class: 'ScheduleDailyRemindersJob',
      job_id: '4a42ed43-4cf2-496e-a180-6829ce4efc06',
      queue_name: 'default',
      arguments: ['household=41', 'schedule=52'],
      outcome: :success,
      duration_ms: 14.5
    )
  end

  it 'maps a completed job without serializing arguments' do
    expect(completed_event.to_h).to include(
      'event.name' => 'job.completed',
      'event.dataset' => 'medtracker.job',
      'event.outcome' => 'success',
      'log.level' => 'info',
      'medtracker.job.id' => '4a42ed43-4cf2-496e-a180-6829ce4efc06',
      'medtracker.job.class' => 'ScheduleDailyRemindersJob',
      'medtracker.job.queue' => 'default'
    )
    expect(completed_event.to_json).not_to include('household=41', 'schedule=52')
  end

  it 'uses a stable failure reason for failed jobs' do
    event = described_class.from(
      job_class: 'ScheduleDailyRemindersJob',
      job_id: 'job-123',
      queue_name: 'default',
      outcome: :failure,
      error_type: RuntimeError
    )

    expect(event.to_h).to include(
      'event.outcome' => 'failure',
      'log.level' => 'error',
      'medtracker.reason' => 'failed',
      'error.type' => 'RuntimeError'
    )
  end
end
