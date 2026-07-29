# frozen_string_literal: true

module Otel
  class DatabaseConnectionPoolMetrics
    GAUGES = {
      size: ['medtracker.db.connection_pool.size', 'Configured database connection pool capacity'],
      busy: ['medtracker.db.connection_pool.in_use', 'Database connections currently checked out'],
      idle: ['medtracker.db.connection_pool.idle', 'Database connections currently idle'],
      waiting: ['medtracker.db.connection_pool.waiting', 'Threads waiting for a database connection']
    }.freeze
    COLLECTOR_METRIC = 'medtracker.db.connection_pool.collector'
    TIMEOUT_METRIC = 'medtracker.db.connection_pool.timeouts'

    class << self
      attr_accessor :current

      def record_timeout(pool)
        current&.record_timeout(pool)
      end
    end

    def initialize(pool_resolver:, meter:)
      @pool_resolver = pool_resolver
      @meter = meter
      @error_log_mutex = Mutex.new
    end

    def install
      install_collector
      install_gauges
      @timeout_counter = meter.create_counter(
        TIMEOUT_METRIC,
        unit: '1',
        description: 'Database connection checkout timeouts'
      )
      self.class.current = self
      self
    end

    def record_timeout(timed_out_pool)
      timeout_counter&.add(1, attributes: pool_attributes(timed_out_pool))
    end

    private

    attr_reader :gauges, :meter, :pool_resolver, :timeout_counter

    def install_collector
      meter.create_observable_gauge(
        COLLECTOR_METRIC,
        callback: -> { collect_pool_metrics },
        unit: '1',
        description: 'Collects database connection pool metrics'
      )
    end

    def install_gauges
      @gauges = GAUGES.transform_values do |name, description|
        meter.create_gauge(
          name,
          unit: '1',
          description:
        )
      end
    end

    def collect_pool_metrics
      connection_pools.count do |connection_pool|
        next false if discarded?(connection_pool)

        record_pool_metrics(connection_pool)
        true
      end
    rescue StandardError => e
      log_collection_failure(e)
      0
    end

    def connection_pools
      pool_resolver.call || []
    end

    def discarded?(connection_pool)
      connection_pool.respond_to?(:discarded?) && connection_pool.discarded?
    end

    def record_pool_metrics(connection_pool)
      attributes = pool_attributes(connection_pool)

      GAUGES.each_key do |stat_key|
        value = pool_stat(connection_pool, stat_key)
        gauges.fetch(stat_key).record(value, attributes:) unless value.nil?
      end
    rescue StandardError => e
      log_collection_failure(e)
      nil
    end

    def pool_stat(connection_pool, stat_key)
      connection_pool.stat.fetch(stat_key)
    rescue StandardError => e
      log_collection_failure(e)
      nil
    end

    def log_collection_failure(error)
      @error_log_mutex.synchronize do
        return if @last_error_logged_at && monotonic_time - @last_error_logged_at < 60

        @last_error_logged_at = monotonic_time
        Rails.logger.warn("OpenTelemetry database pool metrics unavailable: #{error.class}: #{error.message}")
      end
    end

    def monotonic_time
      Process.clock_gettime(Process::CLOCK_MONOTONIC)
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
