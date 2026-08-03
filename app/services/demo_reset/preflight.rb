# frozen_string_literal: true

require 'uri'

module DemoReset
  class Preflight
    APPLICATION_HOST = 'med-tracker-canary.damacus.io'
    DATABASE_HOST = 'med-tracker-canary-rw.home.svc.cluster.local'
    STORAGE_ROOT = '/app/storage'
    DATABASE_ROLE = 'med_tracker_owner'

    def self.from_runtime
      connection = ActiveRecord::Base.connection
      database_hosts = ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).map do |configuration|
        configuration.configuration_hash[:host]
      end
      new(
        demo_mode: DemoMode.enabled?,
        application_url: ENV.fetch('APP_URL', nil),
        database_host: database_hosts,
        storage_root: ENV.fetch('ACTIVE_STORAGE_ROOT', ProductionStorage::DEFAULT_ROOT),
        database_role: connection.select_value('SELECT current_user')
      )
    end

    def initialize(demo_mode:, application_url:, database_host:, storage_root:, database_role:)
      @demo_mode = demo_mode
      @application_url = application_url
      @database_host = database_host
      @storage_root = storage_root
      @database_role = database_role
    end

    def call
      failed_targets = target_checks.reject { |_, valid| valid }.keys
      raise UnsafeTargetError, "demo reset refused: #{failed_targets.join(', ')}" if failed_targets.any?

      { outcome: 'passed', targets: target_checks.keys.map(&:to_s) }
    end

    private

    attr_reader :demo_mode, :application_url, :database_host, :storage_root, :database_role

    def target_checks
      {
        demo_mode: demo_mode,
        application_host: application_host == APPLICATION_HOST,
        database_host: database_hosts_valid?,
        storage_root: storage_root == STORAGE_ROOT,
        database_role: database_role == DATABASE_ROLE
      }
    end

    def application_host
      URI.parse(application_url.to_s).host
    rescue URI::InvalidURIError
      nil
    end

    def database_hosts_valid?
      hosts = Array(database_host)
      hosts.any? && hosts.all? { |host| host == DATABASE_HOST }
    end
  end
end
