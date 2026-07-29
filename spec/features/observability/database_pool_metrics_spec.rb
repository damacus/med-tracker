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

  it 'installs metrics against the live Active Record pools' do
    pool = ActiveRecord::Base.connection_pool
    meter = DatabasePoolMetricsTestSupport::Meter.new

    metrics = Otel::DatabaseConnectionPoolMetrics.new(pool_resolver: -> { [pool] }, meter:)
    metrics.install
    meter.collect

    observations = meter.gauges.transform_values(&:recordings)
    expect(observations).to include(
      'medtracker.db.connection_pool.size' => [[pool.stat.fetch(:size), a_hash_including('db.pool.name', 'db.namespace')]],
      'medtracker.db.connection_pool.in_use' => [[pool.stat.fetch(:busy), a_hash_including('db.pool.name', 'db.namespace')]],
      'medtracker.db.connection_pool.idle' => [[pool.stat.fetch(:idle), a_hash_including('db.pool.name', 'db.namespace')]],
      'medtracker.db.connection_pool.waiting' => [[pool.stat.fetch(:waiting), a_hash_including('db.pool.name', 'db.namespace')]]
    )
  end

  it 'exports one labelled datapoint for each live pool through the metrics SDK' do
    primary_config = instance_double(
      ActiveRecord::DatabaseConfigurations::HashConfig,
      name: 'primary',
      database: 'medtracker_test'
    )
    replica_config = instance_double(
      ActiveRecord::DatabaseConfigurations::HashConfig,
      name: 'replica',
      database: 'medtracker_replica'
    )
    primary_pool = instance_double(
      ActiveRecord::ConnectionAdapters::ConnectionPool,
      stat: { size: 10, busy: 4, idle: 6, waiting: 0 },
      db_config: primary_config,
      discarded?: false
    )
    replica_pool = instance_double(
      ActiveRecord::ConnectionAdapters::ConnectionPool,
      stat: { size: 5, busy: 2, idle: 3, waiting: 0 },
      db_config: replica_config,
      discarded?: false
    )
    metric_reader = OpenTelemetry::SDK::Metrics::Export::InMemoryMetricPullExporter.new
    meter_provider = OpenTelemetry::SDK::Metrics::MeterProvider.new
    meter_provider.add_metric_reader(metric_reader)
    meter = meter_provider.meter('medtracker.database_pool.sdk-export-spec')

    pools = [primary_pool, replica_pool]
    Otel::DatabaseConnectionPoolMetrics.new(pool_resolver: -> { pools }, meter:).install

    metric_reader.pull

    size_metric = metric_reader.metric_snapshots.find { |metric| metric.name == 'medtracker.db.connection_pool.size' }
    expect(size_metric.data_points.map { |point| [point.value, point.attributes] }).to contain_exactly(
      [10, { 'db.pool.name' => 'primary', 'db.namespace' => 'medtracker_test' }],
      [5, { 'db.pool.name' => 'replica', 'db.namespace' => 'medtracker_replica' }]
    )

    pools = [replica_pool]
    metric_reader.reset
    metric_reader.pull

    size_metric = metric_reader.metric_snapshots.find { |metric| metric.name == 'medtracker.db.connection_pool.size' }
    expect(size_metric.data_points.map { |point| [point.value, point.attributes] }).to eq(
      [[5, { 'db.pool.name' => 'replica', 'db.namespace' => 'medtracker_replica' }]]
    )
  end
end
