# frozen_string_literal: true

module DemoReset
  class PrimaryDatabaseReset
    SCHEMA_TABLES = %w[ar_internal_metadata schema_migrations].freeze
    ADVISORY_LOCK_SQL = "SELECT pg_advisory_xact_lock(hashtext('med-tracker-demo-reset'))"

    def initialize(connection: ActiveRecord::Base.connection, baseline_loader: DemoBaseline::Loader.method(:load!))
      @connection = connection
      @baseline_loader = baseline_loader
    end

    def call
      connection.transaction(requires_new: true) do
        connection.execute(ADVISORY_LOCK_SQL)
        truncate_runtime_tables
        baseline_loader.call
      end
    end

    private

    attr_reader :connection, :baseline_loader

    def truncate_runtime_tables
      tables = connection.tables - SCHEMA_TABLES
      quoted_tables = tables.map { |table| connection.quote_table_name(table) }.join(', ')
      connection.execute("TRUNCATE TABLE #{quoted_tables} RESTART IDENTITY CASCADE")
    end
  end
end
