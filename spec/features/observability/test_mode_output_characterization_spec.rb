# frozen_string_literal: true

require 'rails_helper'

class ObservabilityCharacterizationController < ApplicationController
  allow_unauthenticated_access

  def show
    render plain: 'ok'
  end

  def failure
    raise 'characterization request failure'
  end
end

class ObservabilityCharacterizationJob < ApplicationJob
  def perform
    Rails.logger.info('characterization job output')
  end
end

RSpec.describe 'Test-mode observability output characterization', type: :request do
  let(:output) { StringIO.new }
  let(:capture_logger) { ActiveSupport::Logger.new(output) }

  before do
    allow(Observability::CanonicalLogger).to receive(:write) { |event| output.puts(event.to_json) }
  end

  around do |example|
    original_rails_logger = Rails.logger
    original_controller_logger = ActionController::Base.logger
    original_job_logger = ActiveJob::Base.logger
    original_log_subscriber_logger = ActiveSupport::LogSubscriber.logger

    Rails.logger = capture_logger
    ActionController::Base.logger = capture_logger
    ActiveJob::Base.logger = capture_logger
    ActiveSupport::LogSubscriber.logger = capture_logger
    Rails.application.routes.draw do
      get '/observability_characterization', to: 'observability_characterization#show'
      get '/observability_characterization/failure', to: 'observability_characterization#failure'
    end

    example.run
  ensure
    Rails.logger = original_rails_logger
    ActionController::Base.logger = original_controller_logger
    ActiveJob::Base.logger = original_job_logger
    ActiveSupport::LogSubscriber.logger = original_log_subscriber_logger
    Rails.application.reload_routes!
  end

  it 'exposes request completion as a structured record' do
    ActiveSupport::Notifications.subscribed(
      Observability::RequestCompletionSubscriber.method(:call),
      'process_action.action_controller'
    ) do
      get '/observability_characterization', headers: { 'X-Request-Id' => 'test-characterization-request' }
    end

    expect(json_records.count { |record| record['event.name'] == 'http.request.completed' }).to eq(1)
    expect(json_records).to include(
      include(
        'event.name' => 'http.request.completed',
        'log.level' => 'info',
        'medtracker.request.id' => 'test-characterization-request'
      )
    )
  end

  it 'exposes job execution as a structured record' do
    ActiveSupport::Notifications.subscribed(
      Observability::JobCompletionSubscriber.method(:call),
      'perform.active_job'
    ) do
      ObservabilityCharacterizationJob.perform_now
    end

    expect(json_records).to include(
      include('event.dataset' => 'medtracker.job', 'log.level' => 'info')
    )
  end

  it 'preserves a custom event outside request context' do
    Observability::DomainEventPublisher.instrument(
      'take_attempted.med_tracker',
      source_type: 'schedule'
    )

    expect(json_records).to include(
      include('event.name' => 'medication_take.attempted')
    )
  end

  it 'gives warning and error output queryable severity' do
    Observability::DiagnosticEvent.emit(
      component: :opentelemetry,
      reason: :configuration_warning,
      severity: :warn
    )
    Observability::DiagnosticEvent.failure(
      component: :opentelemetry,
      error: RuntimeError.new('private text')
    )

    expect(json_records.pluck('log.level')).to include('warn', 'error')
  end

  it 'records an escaped request exception as a structured failure event' do
    expect do
      ActiveSupport::Notifications.subscribed(
        Observability::RequestCompletionSubscriber.method(:call),
        'process_action.action_controller'
      ) do
        get '/observability_characterization/failure'
      end
    end.to raise_error(RuntimeError, 'characterization request failure')
    expect(json_records).to include(
      include('event.outcome' => 'failure', 'error.type' => 'RuntimeError')
    )
  end

  it 'records subscriber failure before preserving propagation' do
    subscriber = ->(*) { raise 'characterization subscriber failure' }

    expect do
      ActiveSupport::Notifications.subscribed(subscriber, 'take_attempted.med_tracker') do
        Observability::DomainEventPublisher.instrument(
          'take_attempted.med_tracker',
          source_type: 'schedule'
        )
      end
    end.to raise_error(RuntimeError, 'characterization subscriber failure')
    expect(json_records).to include(
      include('event.outcome' => 'failure', 'error.type' => 'RuntimeError')
    )
  end

  it 'correlates valid sampled trace context' do
    expect(Otel::LogCorrelation.options(span: span_with(context: valid_context(sampled: true)))).to eq(
      'trace.id' => '6e0c63257de34c92bf9efcd03927272e',
      'span.id' => 'f7c2f2d910e142be'
    )
  end

  it 'omits invalid trace context' do
    expect(Otel::LogCorrelation.options(span: span_with(context: invalid_context))).to eq({})
  end

  it 'retains identifiers for valid unsampled trace context' do
    expect(Otel::LogCorrelation.options(span: span_with(context: valid_context(sampled: false)))).to eq(
      'trace.id' => '6e0c63257de34c92bf9efcd03927272e',
      'span.id' => 'f7c2f2d910e142be'
    )
  end

  def json_records
    output.string.lines.filter_map do |line|
      JSON.parse(line)
    rescue JSON::ParserError
      nil
    end
  end

  def span_with(context:)
    instance_double(OpenTelemetry::Trace::Span, context:)
  end

  def valid_context(sampled:)
    instance_double(
      OpenTelemetry::Trace::SpanContext,
      valid?: true,
      trace_flags: sampled ? OpenTelemetry::Trace::TraceFlags::SAMPLED : OpenTelemetry::Trace::TraceFlags::DEFAULT,
      hex_trace_id: '6e0c63257de34c92bf9efcd03927272e',
      hex_span_id: 'f7c2f2d910e142be'
    )
  end

  def invalid_context
    instance_double(OpenTelemetry::Trace::SpanContext, valid?: false)
  end
end
