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

    def initialize(pool_resolver:, meter:)
      @pool_resolver = pool_resolver
      @meter = meter
      @error_log_mutex = Mutex.new
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

    def record_timeout(timed_out_pool)
      timeout_counter&.add(1, attributes: pool_attributes(timed_out_pool))
    end

    private

    attr_reader :meter, :pool_resolver, :timeout_counter

    def install_gauges
      GAUGES.each do |stat_key, (name, description)|
        meter.create_observable_gauge(
          name,
          callback: -> { pool_observations(stat_key) },
          unit: '1',
          description:
        )
      end
    end

    def pool_observations(stat_key)
      connection_pools.filter_map do |connection_pool|
        next if discarded?(connection_pool)

        pool_stat(connection_pool, stat_key)
      end
    rescue StandardError => e
      log_collection_failure(e)
      []
    end

    def connection_pools
      pool_resolver.call || []
    end

    def discarded?(connection_pool)
      connection_pool.respond_to?(:discarded?) && connection_pool.discarded?
    end

    def pool_stat(connection_pool, stat_key)
      [connection_pool.stat.fetch(stat_key), pool_attributes(connection_pool)]
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
