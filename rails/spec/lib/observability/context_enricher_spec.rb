# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::ContextEnricher do
  after { Current.reset }

  it 'combines request, job, workflow, and valid trace context' do
    set_current_context
    allow(Otel::LogCorrelation).to receive(:options).and_return(trace_context)

    expect(described_class.call).to include(
      'medtracker.request.id' => 'request-opaque',
      'medtracker.job.id' => 'job-opaque',
      'medtracker.workflow.id' => Current.observability_context.workflow_id,
      'trace.id' => '6e0c63257de34c92bf9efcd03927272e',
      'span.id' => '090e3df5e6c74550'
    )
  end

  it 'omits malformed request, job, and absent trace context' do
    Current.request_id = 'raw request identifier'
    Current.job_id = 'x' * 129
    allow(Otel::LogCorrelation).to receive(:options).and_return({})

    expect(described_class.call).to be_empty
  end

  def set_current_context
    Current.request_id = 'request-opaque'
    Current.job_id = 'job-opaque'
    Current.observability_context = Observability::CorrelationContext.start(
      now: Time.iso8601('2026-07-30T09:30:00Z')
    )
  end

  def trace_context
    {
      'trace.id' => '6e0c63257de34c92bf9efcd03927272e',
      'span.id' => '090e3df5e6c74550'
    }
  end
end
