# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Otel::DatabaseConnectionPoolMetrics do
  subject(:metrics) { described_class.new(pool:, meter:) }

  let(:meter) { DatabasePoolMetricsTestSupport::Meter.new }
  let(:db_config) { instance_double(ActiveRecord::DatabaseConfigurations::HashConfig, name: 'primary', database: 'medtracker_test') }
  let(:pool) do
    instance_double(
      ActiveRecord::ConnectionAdapters::ConnectionPool,
      stat: { size: 10, connections: 7, busy: 4, dead: 0, idle: 3, waiting: 2, checkout_timeout: 5.0 },
      db_config:
    )
  end

  it 'registers the pool gauges and timeout counter with stable names' do
    metrics.install

    expect(meter.gauges.keys).to contain_exactly(
      'medtracker.db.connection_pool.size',
      'medtracker.db.connection_pool.in_use',
      'medtracker.db.connection_pool.idle',
      'medtracker.db.connection_pool.waiting'
    )
    expect(meter.counters.keys).to contain_exactly('medtracker.db.connection_pool.timeouts')
  end

  it 'observes current Rails connection pool statistics' do
    metrics.install

    expect(meter.gauges.transform_values(&:observe)).to eq(
      'medtracker.db.connection_pool.size' => 10,
      'medtracker.db.connection_pool.in_use' => 4,
      'medtracker.db.connection_pool.idle' => 3,
      'medtracker.db.connection_pool.waiting' => 2
    )
  end

  it 'records checkout timeouts with bounded pool metadata' do
    metrics.install

    metrics.record_timeout(pool)

    expect(meter.counters.fetch('medtracker.db.connection_pool.timeouts').recordings).to eq(
      [[1, { 'db.pool.name' => 'primary', 'db.namespace' => 'medtracker_test' }]]
    )
  end

  it 'records and re-raises connection checkout timeout errors' do
    timeout_pool_class = Class.new do
      prepend Otel::ConnectionPoolTimeoutInstrumentation

      def checkout
        raise ActiveRecord::ConnectionTimeoutError
      end

      def db_config
        ActiveRecord::Base.connection_pool.db_config
      end
    end
    metrics.install

    expect { timeout_pool_class.new.checkout }.to raise_error(ActiveRecord::ConnectionTimeoutError)
    expect(meter.counters.fetch('medtracker.db.connection_pool.timeouts').recordings.size).to eq(1)
  end

  it 'returns zero from gauge callbacks when pool statistics are unavailable' do
    allow(pool).to receive(:stat).and_raise(ActiveRecord::ConnectionNotEstablished)
    allow(Rails.logger).to receive(:warn)
    metrics.install

    expect(meter.gauges.values.map(&:observe)).to all(eq(0))
    expect(Rails.logger).to have_received(:warn).with(/database pool metrics unavailable/).at_least(:once)
  end
end
