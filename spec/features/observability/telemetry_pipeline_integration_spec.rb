# frozen_string_literal: true

require 'rails_helper'
require 'otel/allowlisted_span_exporter'

RSpec.describe 'Observability telemetry pipeline integration' do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  it 'emits medication workflow metrics through ActiveSupport notifications' do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
    schedule = schedules(:john_paracetamol)
    user = users(:john)

    payloads = captured_payloads('take_recorded.med_tracker') do
      travel_to Time.current.end_of_day - 1.minute do
        TakeMedicationService.new.call(
          source: schedule,
          amount_override: nil,
          taken_from_medication_id: nil,
          user: user
        )
      end
    end

    expect(payloads).to contain_exactly(
      include(
        source_type: 'schedule',
        medicine_context_class: 'Schedule',
        role: 'owner',
        environment: 'test',
        error: nil
      )
    )
  end

  it 'exports request, job, and error trace spans with only approved attributes' do
    exporter = capture_exporter
    pipeline = Otel::AllowlistedSpanExporter.new(exporter)
    request_span = fake_span('GET /dashboard', request_trace_attributes)
    job_span = fake_span('MedicationReminderJob', job_trace_attributes)
    error_span = fake_span('MedicationTake#create', error_trace_attributes)

    expect(pipeline.export([request_span, job_span, error_span])).to eq(OpenTelemetry::SDK::Trace::Export::SUCCESS)

    expect(exporter.exported.map(&:attributes)).to contain_exactly(
      {
        'http.request.method' => 'GET',
        'http.route' => '/households/:household_slug/dashboard',
        'http.response.status_code' => 200
      },
      {
        'model.name' => 'MedicationReminderJob',
        'model.operation' => 'perform',
        'otel.status_code' => 'OK'
      },
      {
        'model.name' => 'MedicationTake',
        'model.operation' => 'create',
        'model.id_hash' => 'safe-hash',
        'error.type' => 'RuntimeError',
        'exception.escaped' => true
      }
    )
  end

  it 'adds trace and span identifiers to structured log metadata' do
    span = instance_double(OpenTelemetry::Trace::Span)
    context = instance_double(
      OpenTelemetry::Trace::SpanContext,
      valid?: true,
      hex_trace_id: '6e0c63257de34c92bf9efcd03927272e',
      hex_span_id: 'f7c2f2d910e142be'
    )

    allow(span).to receive(:context).and_return(context)

    expect(Otel::LogCorrelation.options(span: span)).to eq(
      'trace.id' => '6e0c63257de34c92bf9efcd03927272e',
      'span.id' => 'f7c2f2d910e142be'
    )
  end

  it 'omits structured log trace metadata when the span context is invalid' do
    span = instance_double(OpenTelemetry::Trace::Span)
    context = instance_double(OpenTelemetry::Trace::SpanContext, valid?: false)

    allow(span).to receive(:context).and_return(context)

    expect(Otel::LogCorrelation.options(span: span)).to eq({})
  end

  def captured_payloads(event_name, &)
    payloads = []
    subscriber = lambda do |*args|
      payloads << ActiveSupport::Notifications::Event.new(*args).payload
    end

    ActiveSupport::Notifications.subscribed(subscriber, event_name, &)
    payloads
  end

  def capture_exporter
    Class.new do
      attr_reader :exported

      def export(span_data, timeout: nil)
        @exported = span_data
        @timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end
    end.new
  end

  def fake_span(name, attributes)
    Struct.new(:name, :attributes).new(name, attributes)
  end

  def request_trace_attributes
    {
      'http.request.method' => 'GET',
      'http.route' => '/households/:household_slug/dashboard',
      'http.response.status_code' => 200,
      'user.email' => 'patient@example.test'
    }
  end

  def job_trace_attributes
    {
      'model.name' => 'MedicationReminderJob',
      'model.operation' => 'perform',
      'otel.status_code' => 'OK',
      'job.arguments' => 'patient name'
    }
  end

  def error_trace_attributes
    {
      'model.name' => 'MedicationTake',
      'model.operation' => 'create',
      'model.id_hash' => 'safe-hash',
      'error.type' => 'RuntimeError',
      'exception.escaped' => true,
      'exception.message' => 'patient-specific failure'
    }
  end
end
