# frozen_string_literal: true

module DemoReset
  class AuxiliaryDatabasesReset
    SCHEMA_TABLES = %w[ar_internal_metadata schema_migrations].freeze

    def initialize(configurations: default_configurations, connection_provider: method(:with_connection))
      @configurations = configurations
      @connection_provider = connection_provider
    end

    def call
      configurations.to_h do |configuration|
        [configuration.name.to_sym, reset(configuration)]
      end
    end

    def verify_empty!
      counts = configurations.to_h do |configuration|
        [configuration.name.to_sym, count_runtime_rows(configuration)]
      end
      raise VerificationError, 'auxiliary_database_not_empty' if counts.values.any?(&:positive?)

      counts
    end

    private

    attr_reader :configurations, :connection_provider

    def default_configurations
      ActiveRecord::Base.configurations
                        .configs_for(env_name: Rails.env)
                        .reject { |configuration| configuration.name == 'primary' }
    end

    def reset(configuration)
      connection_provider.call(configuration) do |connection|
        tables = runtime_tables(connection)
        connection.transaction do
          truncate(connection, tables)
        end
        tables.size
      end
    end

    def count_runtime_rows(configuration)
      connection_provider.call(configuration) do |connection|
        runtime_tables(connection).sum do |table|
          connection.select_value("SELECT COUNT(*) FROM #{connection.quote_table_name(table)}").to_i
        end
      end
    end

    def runtime_tables(connection)
      connection.tables - SCHEMA_TABLES
    end

    def truncate(connection, tables)
      return if tables.empty?

      connection.truncate_tables(*tables)
    end

    def with_connection(configuration)
      connection_class = Class.new(ApplicationRecord) do
        self.abstract_class = true
      end
      connection_class.establish_connection(configuration.configuration_hash)
      yield connection_class.connection
    ensure
      connection_class&.connection_pool&.disconnect!
    end
  end
end
