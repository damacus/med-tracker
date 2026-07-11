# frozen_string_literal: true

module Otel
  class DatabaseConnectionPoolMetrics
    GAUGES = {
      size: ['medtracker.db.connection_pool.size', 'Configured database connection pool capacity'],
      busy: ['medtracker.db.connection_pool.in_use', 'Database connections currently checked out'],
      idle: ['medtracker.db.connection_pool.idle', 'Database connections currently idle'],
      waiting: ['medtracker.db.connection_pool.waiting', 'Threads waiting for a database connection']
    }.freeze
    TIMEOUT_METRIC = 'medtracker.db.connection_pool.timeouts'

    class << self
      attr_accessor :current

      def record_timeout(pool)
        current&.record_timeout(pool)
      end
    end

    def initialize(pool:, meter:)
      @pool = pool
      @meter = meter
    end

    def install
      install_gauges
      @timeout_counter = meter.create_counter(
        TIMEOUT_METRIC,
        unit: '1',
        description: 'Database connection checkout timeouts'
      )
      self.class.current = self
      self
    end

    def record_timeout(timed_out_pool = pool)
      timeout_counter&.add(1, attributes: pool_attributes(timed_out_pool))
    end

    private

    attr_reader :meter, :pool, :timeout_counter

    def install_gauges
      GAUGES.each do |stat_key, (name, description)|
        meter.create_observable_gauge(
          name,
          callback: -> { pool_stat(stat_key) },
          unit: '1',
          description:
        )
      end
    end

    def pool_stat(stat_key)
      pool.stat.fetch(stat_key)
    rescue StandardError => e
      Rails.logger.warn("OpenTelemetry database pool metrics unavailable: #{e.class}: #{e.message}")
      0
    end

    def pool_attributes(connection_pool)
      {
        'db.pool.name' => connection_pool.db_config.name,
        'db.namespace' => connection_pool.db_config.database.to_s
      }
    end
  end

  module ConnectionPoolTimeoutInstrumentation
    def checkout(...)
      super
    rescue ActiveRecord::ConnectionTimeoutError
      DatabaseConnectionPoolMetrics.record_timeout(self)
      raise
    end
  end
end
