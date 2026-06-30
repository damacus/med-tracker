# frozen_string_literal: true

module MedTracker
  class Pre05DatabaseUpgradePreflight
    ROLE_NAMES = %w[med_tracker_owner med_tracker_app].freeze
    UPGRADE_RUNBOOK_URL = 'https://damacus.github.io/med-tracker/pre-0-5-database-upgrade/'

    def initialize(connection)
      @connection = connection
    end

    def call
      role_rows = runtime_role_rows.index_by { |row| row.fetch('rolname') }
      missing_roles = ROLE_NAMES - role_rows.keys

      missing_role_errors(missing_roles) +
        unsafe_role_errors(role_rows) +
        membership_errors(missing_roles)
    end

    private

    attr_reader :connection

    def runtime_role_rows
      connection.select_all(<<~SQL.squish).to_a
        SELECT rolname, rolcanlogin, rolsuper, rolbypassrls
        FROM pg_roles
        WHERE rolname IN ('med_tracker_owner', 'med_tracker_app')
        ORDER BY rolname
      SQL
    end

    def missing_role_errors(missing_roles)
      missing_roles.map { |role_name| "Missing PostgreSQL runtime role: #{role_name}" }
    end

    def unsafe_role_errors(role_rows)
      ROLE_NAMES.filter_map do |role_name|
        role_row = role_rows[role_name]
        next if role_row.blank?

        unsafe_attributes = unsafe_attributes_for(role_row)
        next if unsafe_attributes.empty?

        "#{role_name} has unsafe attributes: #{unsafe_attributes.join(', ')}"
      end
    end

    def unsafe_attributes_for(role_row)
      attributes = []
      attributes << 'LOGIN' if truthy?(role_row.fetch('rolcanlogin'))
      attributes << 'SUPERUSER' if truthy?(role_row.fetch('rolsuper'))
      attributes << 'BYPASSRLS' if truthy?(role_row.fetch('rolbypassrls'))
      attributes
    end

    def membership_errors(missing_roles)
      (ROLE_NAMES - missing_roles).filter_map do |role_name|
        next if truthy?(runtime_role_member?(role_name))

        "Current database login is not a member of #{role_name}"
      end
    end

    def runtime_role_member?(role_name)
      connection.select_value(<<~SQL.squish)
        SELECT pg_has_role(session_user, #{connection.quote(role_name)}, 'member')
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
