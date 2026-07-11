# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Database pool metrics export pipeline' do
  it 'loads the OTLP metrics exporter used by runtime environments' do
    require 'opentelemetry-exporter-otlp-metrics'

    expect(OpenTelemetry::Exporter::OTLP::Metrics::MetricsExporter).to be_a(Class)
  end

  it 'uses the OpenTelemetry metrics SDK provider in the test runtime' do
    expect(OpenTelemetry.meter_provider).to be_a(OpenTelemetry::SDK::Metrics::MeterProvider)
  end

  it 'installs metrics against the runtime Active Record pool' do
    pool = ActiveRecord::Base.connection_pool
    meter = DatabasePoolMetricsTestSupport::Meter.new

    metrics = Otel::DatabaseConnectionPoolMetrics.new(pool:, meter:)
    metrics.install

    observations = meter.gauges.transform_values(&:observe)
    expect(observations).to include(
      'medtracker.db.connection_pool.size' => pool.stat.fetch(:size),
      'medtracker.db.connection_pool.in_use' => pool.stat.fetch(:busy),
      'medtracker.db.connection_pool.idle' => pool.stat.fetch(:idle),
      'medtracker.db.connection_pool.waiting' => pool.stat.fetch(:waiting)
    )
  end
end
