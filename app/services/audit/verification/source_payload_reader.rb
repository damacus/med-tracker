# frozen_string_literal: true

module Audit
  module Verification
    class SourcePayloadReader
      SOURCE_SQL = {
        'security_audit_events' => <<~SQL.squish,
          SELECT (to_jsonb(source_row.*) - 'updated_at')::text
          FROM security_audit_events source_row
          WHERE source_row.id = $1
            AND source_row.household_id IS NOT DISTINCT FROM $2
        SQL
        'versions' => <<~SQL.squish
          SELECT (to_jsonb(source_row.*) - 'updated_at')::text
          FROM versions source_row
          WHERE source_row.id = $1
            AND source_row.household_id IS NOT DISTINCT FROM $2
        SQL
      }.freeze

      def call(entry)
        value = connection.select_value(SOURCE_SQL.fetch(entry.source_table), 'Audit source', binds(entry))
        JSON.parse(value) if value
      end

      private

      def binds(entry)
        [query_attribute('source_id', entry.source_id),
         query_attribute('source_household_id', entry.household_id, type: 'household_id')]
      end

      def query_attribute(name, value, type: name)
        ActiveRecord::Relation::QueryAttribute.new(name, value, AuditLedgerEntry.type_for_attribute(type))
      end

      def connection
        ActiveRecord::Base.connection
      end
    end
  end
end
