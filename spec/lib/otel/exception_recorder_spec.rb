# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Otel::ExceptionRecorder do
  describe '.record' do
    it 'records escaped exceptions on the current valid span' do
      span = instance_double(OpenTelemetry::Trace::Span)
      context = instance_double(OpenTelemetry::Trace::SpanContext, valid?: true)
      exception = RuntimeError.new('boom')

      allow(span).to receive(:context).and_return(context)
      allow(span).to receive(:record_exception)
      allow(span).to receive(:set_attribute)
      allow(span).to receive(:status=)
      allow(OpenTelemetry::Trace).to receive(:current_span).and_return(span)

      described_class.record(exception, source: 'request')

      expect_exception_recorded(span, exception)
      expect_error_attributes_set(span)
      expect(span).to have_received(:status=).with(an_instance_of(OpenTelemetry::Trace::Status))
    end

    it 'does not record when the current span context is invalid' do
      span = instance_double(OpenTelemetry::Trace::Span)
      context = instance_double(OpenTelemetry::Trace::SpanContext, valid?: false)
      exception = RuntimeError.new('boom')

      allow(span).to receive(:context).and_return(context)
      allow(span).to receive(:record_exception)
      allow(OpenTelemetry::Trace).to receive(:current_span).and_return(span)

      described_class.record(exception, source: 'job')

      expect(span).not_to have_received(:record_exception)
    end
  end

  def expect_exception_recorded(span, exception)
    expect(span).to have_received(:record_exception).with(
      exception,
      attributes: {
        'error.type' => 'RuntimeError',
        'exception.escaped' => true,
        'exception.source' => 'request'
      }
    )
  end

  def expect_error_attributes_set(span)
    expect(span).to have_received(:set_attribute).with('error.type', 'RuntimeError')
    expect(span).to have_received(:set_attribute).with('exception.escaped', true)
    expect(span).to have_received(:set_attribute).with('exception.source', 'request')
  end
end
