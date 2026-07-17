# frozen_string_literal: true

module Audit
  module Verification
    class DatabaseAuthority
      REQUIRED_SELECT_TABLES = %w[
        audit_chain_heads audit_ledger_entries audit_signing_keys audit_checkpoints security_audit_events versions
      ].freeze
      VERIFIER_ROLE = 'med_tracker_audit_verifier'
      VERIFIER_POLICY = 'audit_verifier_complete_visibility'

      def initialize(connection: ActiveRecord::Base.connection)
        @connection = connection
      end

      def verify!
        raise ConfigurationError, "database verification requires #{VERIFIER_ROLE}" unless current_user == VERIFIER_ROLE
        raise ConfigurationError, 'database verifier role has unsafe row-security authority' unless verifier_role_safe?

        missing_tables = REQUIRED_SELECT_TABLES.reject { |table_name| select_privilege?(table_name) }
        if missing_tables.any?
          raise ConfigurationError, "database verifier lacks required SELECT privilege: #{missing_tables.join(', ')}"
        end
        return if complete_visibility_policy?

        raise ConfigurationError, 'database verifier complete RLS policy is missing or invalid'
      end

      private

      attr_reader :connection

      def current_user
        connection.select_value('SELECT current_user')
      end

      def verifier_role_safe?
        connection.select_value(<<~SQL.squish)
          SELECT NOT rolsuper AND NOT rolbypassrls
          FROM pg_roles
          WHERE rolname = current_user
        SQL
      end

      def select_privilege?(table_name)
        connection.select_value(<<~SQL.squish)
          SELECT has_table_privilege(current_user, #{connection.quote(table_name)}, 'SELECT')
        SQL
      end

      def complete_visibility_policy?
        connection.select_value(<<~SQL.squish)
          SELECT relations.relrowsecurity AND relations.relforcerowsecurity AND COUNT(policies.*) = 1
          FROM pg_class relations
          LEFT JOIN pg_policies policies
            ON policies.schemaname = 'public'
            AND policies.tablename = relations.relname
            AND policies.policyname = #{connection.quote(VERIFIER_POLICY)}
            AND policies.permissive = 'PERMISSIVE'
            AND policies.roles = ARRAY[#{connection.quote(VERIFIER_ROLE)}]::name[]
            AND policies.cmd = 'SELECT'
            AND policies.qual = 'true'
            AND policies.with_check IS NULL
          WHERE relations.oid = 'security_audit_events'::regclass
          GROUP BY relations.relrowsecurity, relations.relforcerowsecurity
        SQL
      end
    end
  end
end
