# frozen_string_literal: true

module Audit
  module Verification
    class DatabaseAuthority
      REQUIRED_SELECT_TABLES = %w[
        audit_chain_heads audit_ledger_entries audit_signing_keys audit_checkpoints security_audit_events versions
      ].freeze
      APPROVED_SELECT_TABLES = (REQUIRED_SELECT_TABLES + %w[audit_export_deliveries]).freeze
      FORBIDDEN_ROLES = %w[med_tracker_app med_tracker_owner med_tracker_audit_exporter].freeze
      MUTATION_PRIVILEGES = %w[INSERT UPDATE DELETE TRUNCATE REFERENCES TRIGGER].freeze
      COLUMN_MUTATION_PRIVILEGES = %w[INSERT UPDATE REFERENCES].freeze
      VERIFIER_ROLE = 'med_tracker_audit_verifier'
      EXPECTED_SECURITY_EVENT_POLICIES = [
        ['audit_verifier_complete_visibility', 'PERMISSIVE', '{med_tracker_audit_verifier}', 'SELECT', 'true', nil],
        [
          'household_tenant_isolation', 'PERMISSIVE', '{public}', 'ALL',
          '(household_id = med_tracker.current_household_id())',
          '(household_id = med_tracker.current_household_id())'
        ]
      ].freeze

      def initialize(connection: ActiveRecord::Base.connection)
        @connection = connection
      end

      def verify!
        raise ConfigurationError, "database verification requires #{VERIFIER_ROLE}" unless current_user == VERIFIER_ROLE
        raise ConfigurationError, 'database verifier role has unsafe row-security authority' unless verifier_role_safe?

        verify_forbidden_role_memberships!

        missing_tables = REQUIRED_SELECT_TABLES.reject { |table_name| select_privilege?(table_name) }
        if missing_tables.any?
          raise ConfigurationError, "database verifier lacks required SELECT privilege: #{missing_tables.join(', ')}"
        end

        verify_mutation_privileges!
        verify_unapproved_select_privileges!
        verify_security_event_policy_set!
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

      def verify_forbidden_role_memberships!
        memberships = connection.select_values(
          <<~SQL.squish,
            SELECT role_name
            FROM jsonb_array_elements_text($1::jsonb) AS forbidden(role_name)
            WHERE pg_has_role(current_user, role_name, 'MEMBER')
          SQL
          'Audit forbidden role memberships',
          [text_array_bind('forbidden_roles', FORBIDDEN_ROLES)]
        )
        return if memberships.empty?

        raise ConfigurationError, "database verifier has forbidden role membership: #{memberships.join(', ')}"
      end

      def verify_mutation_privileges!
        privileges = table_mutation_privileges + column_mutation_privileges + sequence_mutation_privileges +
                     audit_function_privileges
        privileges = privileges.uniq.sort
        return if privileges.empty?

        raise ConfigurationError, "database verifier has effective audit mutation privilege: #{privileges.join(', ')}"
      end

      def table_mutation_privileges
        connection.select_values(
          <<~SQL.squish,
            SELECT DISTINCT relations.relname || ':' || privileges.name
            FROM pg_class relations
            JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
            CROSS JOIN jsonb_array_elements_text($1::jsonb) AS privileges(name)
            WHERE namespaces.nspname = 'public'
              AND relations.relname IN (SELECT jsonb_array_elements_text($2::jsonb))
              AND has_table_privilege(current_user, relations.oid, privileges.name)
            ORDER BY 1
          SQL
          'Audit table mutation privileges',
          text_array_binds(mutation_privileges: MUTATION_PRIVILEGES, approved_tables: APPROVED_SELECT_TABLES)
        )
      end

      def column_mutation_privileges
        connection.select_values(
          <<~SQL.squish,
            SELECT DISTINCT relations.relname || ':column-' || privileges.name
            FROM pg_class relations
            JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
            CROSS JOIN jsonb_array_elements_text($1::jsonb) AS privileges(name)
            WHERE namespaces.nspname = 'public'
              AND relations.relname IN (SELECT jsonb_array_elements_text($2::jsonb))
              AND has_any_column_privilege(current_user, relations.oid, privileges.name)
              AND NOT has_table_privilege(current_user, relations.oid, privileges.name)
            ORDER BY 1
          SQL
          'Audit column mutation privileges',
          text_array_binds(
            column_mutation_privileges: COLUMN_MUTATION_PRIVILEGES,
            approved_tables: APPROVED_SELECT_TABLES
          )
        )
      end

      def sequence_mutation_privileges
        connection.select_values(
          <<~SQL.squish,
            SELECT relations.relname || ':sequence-mutation'
            FROM pg_class relations
            JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
            WHERE namespaces.nspname = 'public'
              AND relations.relkind = 'S'
              AND relations.relname IN (SELECT jsonb_array_elements_text($1::jsonb))
              AND (
                has_sequence_privilege(current_user, relations.oid, 'USAGE')
                OR has_sequence_privilege(current_user, relations.oid, 'UPDATE')
              )
            ORDER BY 1
          SQL
          'Audit sequence mutation privileges',
          [text_array_bind('approved_sequences', APPROVED_SELECT_TABLES.map { |name| "#{name}_id_seq" })]
        )
      end

      def audit_function_privileges
        connection.select_values(<<~SQL.squish)
          SELECT procedures.oid::regprocedure::text || ':execute'
          FROM pg_proc procedures
          JOIN pg_namespace namespaces ON namespaces.oid = procedures.pronamespace
          WHERE namespaces.nspname = 'public'
            AND procedures.proname LIKE 'audit_%'
            AND has_function_privilege(current_user, procedures.oid, 'EXECUTE')
          ORDER BY 1
        SQL
      end

      def verify_unapproved_select_privileges!
        tables = connection.select_values(
          <<~SQL.squish,
            SELECT relations.relname
            FROM pg_class relations
            JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
            WHERE namespaces.nspname = 'public'
              AND relations.relkind IN ('r', 'p', 'v', 'm', 'f')
              AND relations.relname NOT IN (SELECT jsonb_array_elements_text($1::jsonb))
              AND has_any_column_privilege(current_user, relations.oid, 'SELECT')
            ORDER BY relations.relname
          SQL
          'Audit unapproved select privileges',
          [text_array_bind('approved_tables', APPROVED_SELECT_TABLES)]
        )
        return if tables.empty?

        raise ConfigurationError, "database verifier has unapproved SELECT privilege: #{tables.join(', ')}"
      end

      def verify_security_event_policy_set!
        policy_rows = connection.select_rows(<<~SQL.squish)
          SELECT policyname, permissive, roles::text, cmd, qual, with_check
          FROM pg_policies
          WHERE schemaname = 'public'
            AND tablename = 'security_audit_events'
          ORDER BY policyname
        SQL
        return if security_events_forced_rls? && policy_rows == EXPECTED_SECURITY_EVENT_POLICIES

        raise ConfigurationError, 'database verifier security-event RLS policy set is invalid'
      end

      def security_events_forced_rls?
        connection.select_value(<<~SQL.squish)
          SELECT relrowsecurity AND relforcerowsecurity
          FROM pg_class
          WHERE oid = 'security_audit_events'::regclass
        SQL
      end

      def text_array_binds(values_by_name)
        values_by_name.map { |name, values| text_array_bind(name.to_s, values) }
      end

      def text_array_bind(name, values)
        ActiveRecord::Relation::QueryAttribute.new(name, values.to_json, ActiveRecord::Type::String.new)
      end
    end
  end
end
