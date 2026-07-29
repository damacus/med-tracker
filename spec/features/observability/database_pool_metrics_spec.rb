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
    expect(observations).to include(pool_observations(pool))
  end

  it 'exports one labelled datapoint for each live pool through the metrics SDK' do
    metric_reader, _pools = configured_sdk_metrics
    metric_reader.pull

    expect(size_data_points(metric_reader)).to contain_exactly(
      [10, { 'db.pool.name' => 'primary', 'db.namespace' => 'medtracker_test' }],
      [5, { 'db.pool.name' => 'replica', 'db.namespace' => 'medtracker_replica' }]
    )
  end

  it 'exports a numeric collector datapoint that the OTLP exporter can encode' do
    metric_reader, _pools = configured_sdk_metrics
    metric_reader.pull

    collector = latest_metric(metric_reader, 'medtracker.db.connection_pool.collector')
    encoded_metrics = OpenTelemetry::Exporter::OTLP::Metrics::MetricsExporter.allocate.send(
      :encode,
      metric_reader.metric_snapshots
    )

    expect(collector.data_points.map(&:value)).to eq([2])
    expect(encoded_metrics).to be_present
  end

  it 'removes a pool that is no longer live from the next export' do
    metric_reader, pools = configured_sdk_metrics
    metric_reader.pull

    pools.replace([pools.last])
    metric_reader.pull

    expect(size_data_points(metric_reader)).to eq(
      [[5, { 'db.pool.name' => 'replica', 'db.namespace' => 'medtracker_replica' }]]
    )
  end

  def pool_observations(pool)
    attributes = a_hash_including('db.pool.name', 'db.namespace')

    {
      'medtracker.db.connection_pool.size' => [[pool.stat.fetch(:size), attributes]],
      'medtracker.db.connection_pool.in_use' => [[pool.stat.fetch(:busy), attributes]],
      'medtracker.db.connection_pool.idle' => [[pool.stat.fetch(:idle), attributes]],
      'medtracker.db.connection_pool.waiting' => [[pool.stat.fetch(:waiting), attributes]]
    }
  end

  def configured_sdk_metrics
    pools = [
      build_pool('primary', 'medtracker_test', 10, 4, 6),
      build_pool('replica', 'medtracker_replica', 5, 2, 3)
    ]
    metric_reader = OpenTelemetry::SDK::Metrics::Export::InMemoryMetricPullExporter.new
    meter_provider = OpenTelemetry::SDK::Metrics::MeterProvider.new
    meter_provider.add_metric_reader(metric_reader)
    meter = meter_provider.meter('medtracker.database_pool.sdk-export-spec')

    Otel::DatabaseConnectionPoolMetrics.new(pool_resolver: -> { pools }, meter:).install
    [metric_reader, pools]
  end

  def build_pool(name, database, size, busy, idle)
    db_config = instance_double(ActiveRecord::DatabaseConfigurations::HashConfig, name:, database:)

    instance_double(
      ActiveRecord::ConnectionAdapters::ConnectionPool,
      stat: { size:, busy:, idle:, waiting: 0 },
      db_config:,
      discarded?: false
    )
  end

  def size_data_points(metric_reader)
    metric = latest_metric(metric_reader, 'medtracker.db.connection_pool.size')
    metric.data_points.map { |point| [point.value, point.attributes] }
  end

  def latest_metric(metric_reader, name)
    metric_reader.metric_snapshots.rfind { |snapshot| snapshot.name == name }
  end
end
