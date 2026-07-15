# frozen_string_literal: true

module MedTracker
  class Pre05DatabaseUpgradePreflight
    ROLE_LOGIN_EXPECTATIONS = {
      'med_tracker_owner' => false,
      'med_tracker_app' => false,
      'medtracker_auxiliary' => true,
      'medtracker_migration' => true,
      'medtracker_runtime' => true
    }.freeze
    EXPECTED_MEMBERSHIPS = [
      ['med_tracker_app', 'medtracker_runtime', false, true],
      ['med_tracker_owner', 'medtracker_migration', false, true]
    ].freeze
    UNSAFE_ROLE_ATTRIBUTES = {
      'rolsuper' => 'SUPERUSER',
      'rolcreaterole' => 'CREATEROLE',
      'rolcreatedb' => 'CREATEDB',
      'rolreplication' => 'REPLICATION',
      'rolbypassrls' => 'BYPASSRLS'
    }.freeze
    UPGRADE_RUNBOOK_URL = 'https://damacus.github.io/med-tracker/pre-0-5-database-upgrade/'

    def initialize(connection)
      @connection = connection
    end

    def call
      role_rows = runtime_role_rows.index_by { |row| row.fetch('rolname') }
      missing_roles = ROLE_LOGIN_EXPECTATIONS.keys - role_rows.keys

      missing_role_errors(missing_roles) +
        unsafe_role_errors(role_rows) +
        membership_errors +
        current_login_errors
    end

    private

    attr_reader :connection

    def runtime_role_rows
      connection.select_all(<<~SQL.squish).to_a
        SELECT rolname, rolcanlogin, rolsuper, rolcreaterole, rolcreatedb, rolreplication, rolbypassrls
        FROM pg_roles
        WHERE rolname IN (
          'med_tracker_owner',
          'med_tracker_app',
          'medtracker_auxiliary',
          'medtracker_migration',
          'medtracker_runtime'
        )
        ORDER BY rolname
      SQL
    end

    def missing_role_errors(missing_roles)
      missing_roles.map { |role_name| "Missing PostgreSQL runtime role: #{role_name}" }
    end

    def unsafe_role_errors(role_rows)
      ROLE_LOGIN_EXPECTATIONS.filter_map do |role_name, login_expected|
        role_row = role_rows[role_name]
        next if role_row.blank?

        unsafe_attributes = unsafe_attributes_for(role_row, login_expected)
        next if unsafe_attributes.empty?

        "#{role_name} has unsafe attributes: #{unsafe_attributes.join(', ')}"
      end
    end

    def unsafe_attributes_for(role_row, login_expected)
      [login_attribute_error(role_row, login_expected), *unsafe_privilege_attributes(role_row)].compact
    end

    def login_attribute_error(role_row, login_expected)
      return if truthy?(role_row.fetch('rolcanlogin')) == login_expected

      login_expected ? 'NOLOGIN' : 'LOGIN'
    end

    def unsafe_privilege_attributes(role_row)
      UNSAFE_ROLE_ATTRIBUTES.filter_map do |attribute, label|
        label if truthy?(role_row.fetch(attribute))
      end
    end

    def membership_errors
      return [] if role_memberships == EXPECTED_MEMBERSHIPS

      ['Database logins do not have isolated role membership']
    end

    def role_memberships
      connection.select_all(role_membership_query).map do |row|
        [
          row.fetch('granted_role'),
          row.fetch('member_role'),
          truthy?(row.fetch('inherit_option')),
          truthy?(row.fetch('set_option'))
        ]
      end
    end

    def current_login_errors
      return [] if connection.select_value('SELECT session_user') == 'medtracker_migration'

      ['Preflight must connect through medtracker_migration']
    end

    def role_membership_query
      <<~SQL.squish
        SELECT granted.rolname AS granted_role,
               member.rolname AS member_role,
               memberships.inherit_option,
               memberships.set_option
        FROM pg_auth_members memberships
        JOIN pg_roles granted ON granted.oid = memberships.roleid
        JOIN pg_roles member ON member.oid = memberships.member
        WHERE granted.rolname IN ('med_tracker_owner', 'med_tracker_app')
          AND member.rolname IN ('medtracker_auxiliary', 'medtracker_migration', 'medtracker_runtime')
        ORDER BY granted.rolname, member.rolname
      SQL
    end

    def truthy?(value)
      value == true || %w[1 t true].include?(value.to_s)
    end
  end
end

namespace :med_tracker do
  desc 'Check pre-0.5 database runtime role bootstrap state'
  task pre_0_5_database_upgrade_preflight: :environment do
    errors = MedTracker::Pre05DatabaseUpgradePreflight.new(ActiveRecord::Base.connection).call

    if errors.empty?
      puts 'Pre-0.5 database upgrade preflight passed.'
    else
      abort <<~MESSAGE
        Pre-0.5 database upgrade preflight failed:
        #{errors.map { |error| "- #{error}" }.join("\n")}

        Run the pre-0.5 database upgrade bootstrap first:
        #{MedTracker::Pre05DatabaseUpgradePreflight::UPGRADE_RUNBOOK_URL}
      MESSAGE
    end
  end
end
