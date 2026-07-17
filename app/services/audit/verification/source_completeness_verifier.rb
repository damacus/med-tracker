# frozen_string_literal: true

module Audit
  module Verification
    class SourceCompletenessVerifier
      SOURCE_TABLES = %w[security_audit_events versions].freeze

      def initialize(household_id: nil, connection: ActiveRecord::Base.connection)
        @household_id = household_id
        @connection = connection
      end

      def call
        SOURCE_TABLES.filter_map do |source_table|
          missing_count = incomplete_source_count(source_table)
          next if missing_count.zero?

          Issue.new(
            code: 'source_ledger_entry_missing',
            message: 'source rows do not have exactly one ledger entry',
            chain_key: nil,
            sequence: nil,
            metadata: { source_table:, missing_count: }
          )
        end
      end

      private

      attr_reader :connection, :household_id

      def incomplete_source_count(source_table)
        household_predicate = household_id ? 'WHERE source_row.household_id = $1' : ''
        binds = household_id ? [household_id_bind] : []
        sql = <<~SQL.squish
          SELECT COUNT(*)
          FROM (
            SELECT source_row.id
            FROM #{source_table} source_row
            LEFT JOIN audit_ledger_entries ledger_entry
              ON ledger_entry.source_table = #{connection.quote(source_table)}
              AND ledger_entry.source_id = source_row.id
              AND ledger_entry.household_id IS NOT DISTINCT FROM source_row.household_id
            #{household_predicate}
            GROUP BY source_row.id
            HAVING COUNT(ledger_entry.id) <> 1
          ) incomplete_sources
        SQL
        connection.select_value(sql, 'Audit source completeness', binds).to_i
      end

      def household_id_bind
        ActiveRecord::Relation::QueryAttribute.new(
          'household_id', household_id, AuditLedgerEntry.type_for_attribute('household_id')
        )
      end
    end
  end
end
