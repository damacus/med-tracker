require 'uri'

module Canary
  class MobileClientRegistration
    APPLICATION_HOST = 'med-tracker-canary.damacus.io'.freeze
    DATABASE_HOST = 'med-tracker-canary-rw.home.svc.cluster.local'.freeze
    DATABASE_ROLE = 'med_tracker_owner'.freeze

    def initialize(targets: nil, connection: ActiveRecord::Base.connection)
      @connection = connection
      @targets = targets || self.class.runtime_targets(connection)
    end

    def self.runtime_targets(connection)
      {
        demo_mode: DemoMode.enabled?,
        application_url: ENV.fetch('APP_URL', nil),
        database_hosts: ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).map do |configuration|
          configuration.configuration_hash[:host]
        end,
        database_role: connection.select_value('SELECT current_user')
      }
    end

    def call
      verify_target!
      connection.transaction(requires_new: true) do
        connection.execute(DemoReset::PrimaryDatabaseReset::ADVISORY_LOCK_SQL)
        DemoBaseline::PublicMobileClients.register!
      end
      { outcome: 'succeeded', clients: DemoBaseline::PublicMobileClients::DEFINITIONS.length }
    end

    private

    attr_reader :connection, :targets

    def verify_target!
      checks = {
        demo_mode: targets.fetch(:demo_mode),
        application_url: application_host == APPLICATION_HOST,
        database_hosts: database_hosts_valid?,
        database_role: targets.fetch(:database_role) == DATABASE_ROLE
      }
      failed = checks.reject { |_, valid| valid }.keys
      raise DemoReset::UnsafeTargetError, "mobile registration refused: #{failed.join(', ')}" if failed.any?
    end

    def database_hosts_valid?
      hosts = Array(targets.fetch(:database_hosts))
      hosts.any? && hosts.all?(DATABASE_HOST)
    end

    def application_host
      uri = URI.parse(targets.fetch(:application_url).to_s)
      uri.host if uri.is_a?(URI::HTTPS)
    rescue URI::InvalidURIError
      nil
    end
  end
end
