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
      VERIFIER_POLICY = 'audit_verifier_complete_visibility'

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
        unless complete_visibility_policy?
          raise ConfigurationError, 'database verifier complete RLS policy is missing or invalid'
        end

        verify_competing_visibility_policies!
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
        memberships = connection.select_values(<<~SQL.squish)
          SELECT role_name
          FROM unnest(ARRAY[#{quoted_list(FORBIDDEN_ROLES)}]::text[]) AS forbidden(role_name)
          WHERE pg_has_role(current_user, role_name, 'MEMBER')
        SQL
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
        connection.select_values(<<~SQL.squish)
          SELECT DISTINCT relations.relname || ':' || privileges.name
          FROM pg_class relations
          JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
          CROSS JOIN unnest(ARRAY[#{quoted_list(MUTATION_PRIVILEGES)}]::text[]) AS privileges(name)
          WHERE namespaces.nspname = 'public'
            AND relations.relname IN (#{quoted_list(APPROVED_SELECT_TABLES)})
            AND has_table_privilege(current_user, relations.oid, privileges.name)
          ORDER BY 1
        SQL
      end

      def column_mutation_privileges
        connection.select_values(<<~SQL.squish)
          SELECT DISTINCT relations.relname || ':column-' || privileges.name
          FROM pg_class relations
          JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
          CROSS JOIN unnest(ARRAY[#{quoted_list(COLUMN_MUTATION_PRIVILEGES)}]::text[]) AS privileges(name)
          WHERE namespaces.nspname = 'public'
            AND relations.relname IN (#{quoted_list(APPROVED_SELECT_TABLES)})
            AND has_any_column_privilege(current_user, relations.oid, privileges.name)
            AND NOT has_table_privilege(current_user, relations.oid, privileges.name)
          ORDER BY 1
        SQL
      end

      def sequence_mutation_privileges
        connection.select_values(<<~SQL.squish)
          SELECT relations.relname || ':sequence-mutation'
          FROM pg_class relations
          JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
          WHERE namespaces.nspname = 'public'
            AND relations.relkind = 'S'
            AND relations.relname IN (#{quoted_list(APPROVED_SELECT_TABLES.map { |name| "#{name}_id_seq" })})
            AND (
              has_sequence_privilege(current_user, relations.oid, 'USAGE')
              OR has_sequence_privilege(current_user, relations.oid, 'UPDATE')
            )
          ORDER BY 1
        SQL
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
        tables = connection.select_values(<<~SQL.squish)
          SELECT relations.relname
          FROM pg_class relations
          JOIN pg_namespace namespaces ON namespaces.oid = relations.relnamespace
          WHERE namespaces.nspname = 'public'
            AND relations.relkind IN ('r', 'p', 'v', 'm', 'f')
            AND relations.relname NOT IN (#{quoted_list(APPROVED_SELECT_TABLES)})
            AND has_any_column_privilege(current_user, relations.oid, 'SELECT')
          ORDER BY relations.relname
        SQL
        return if tables.empty?

        raise ConfigurationError, "database verifier has unapproved SELECT privilege: #{tables.join(', ')}"
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

      def verify_competing_visibility_policies!
        policies = connection.select_values(<<~SQL.squish)
          SELECT policyname
          FROM pg_policies
          WHERE schemaname = 'public'
            AND tablename = 'security_audit_events'
            AND permissive = 'PERMISSIVE'
            AND cmd IN ('SELECT', 'ALL')
            AND qual = 'true'
            AND roles <> ARRAY[#{connection.quote(VERIFIER_ROLE)}]::name[]
          ORDER BY policyname
        SQL
        return if policies.empty?

        raise ConfigurationError, "database verifier has competing unrestricted RLS policy: #{policies.join(', ')}"
      end

      def quoted_list(values)
        values.map { |value| connection.quote(value) }.join(', ')
      end
    end
  end
end
