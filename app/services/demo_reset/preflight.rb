# frozen_string_literal: true

require 'uri'

module DemoReset
  class Preflight
    Targets = Data.define(
      :demo_mode, :application_url, :database_host, :storage_service, :storage_endpoint, :storage_bucket, :database_role
    )

    APPLICATION_HOST = 'med-tracker-canary.damacus.io'
    DATABASE_HOST = 'med-tracker-canary-rw.home.svc.cluster.local'
    STORAGE_SERVICE = 's3'
    STORAGE_ENDPOINT = 'http://rustfs.storage.svc.cluster.local:9000'
    STORAGE_BUCKET = 'med-tracker-canary'
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
        storage_service: ENV.fetch('ACTIVE_STORAGE_SERVICE', nil),
        storage_endpoint: ENV.fetch('ACTIVE_STORAGE_S3_ENDPOINT', nil),
        storage_bucket: ENV.fetch('ACTIVE_STORAGE_S3_BUCKET', nil),
        database_role: connection.select_value('SELECT current_user')
      )
    end

    def initialize(**targets)
      @targets = Targets.new(**targets)
    end

    def call
      failed_targets = target_checks.reject { |_, valid| valid }.keys
      raise UnsafeTargetError, "demo reset refused: #{failed_targets.join(', ')}" if failed_targets.any?

      { outcome: 'passed', targets: target_checks.keys.map(&:to_s) }
    end

    private

    attr_reader :targets

    def target_checks
      {
        demo_mode: targets.demo_mode,
        application_host: application_host == APPLICATION_HOST,
        database_host: database_hosts_valid?,
        storage_service: targets.storage_service == STORAGE_SERVICE,
        storage_endpoint: targets.storage_endpoint == STORAGE_ENDPOINT,
        storage_bucket: targets.storage_bucket == STORAGE_BUCKET,
        database_role: targets.database_role == DATABASE_ROLE
      }
    end

    def application_host
      URI.parse(targets.application_url.to_s).host
    rescue URI::InvalidURIError
      nil
    end

    def database_hosts_valid?
      hosts = Array(targets.database_host)
      hosts.any? && hosts.all? { |host| host == DATABASE_HOST }
    end
  end
end
