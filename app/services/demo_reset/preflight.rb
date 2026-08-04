# frozen_string_literal: true

require 'uri'

module DemoReset
  class Preflight
    Targets = Data.define(
      :demo_mode, :application_url, :database_host, :storage_service, :storage_root, :storage_endpoint,
      :storage_bucket, :database_role
    )
    ExpectedTargets = Data.define(
      :application_host, :database_host, :storage_service, :storage_root, :storage_endpoint, :storage_bucket,
      :database_role
    )

    def self.from_runtime
      connection = ActiveRecord::Base.connection
      database_hosts = ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).map do |configuration|
        configuration.configuration_hash[:host]
      end
      new(
        expected: expected_targets_from_runtime,
        demo_mode: DemoMode.enabled?,
        application_url: ENV.fetch('APP_URL', nil),
        database_host: database_hosts,
        storage_service: ENV.fetch('ACTIVE_STORAGE_SERVICE', nil),
        storage_root: ENV.fetch('ACTIVE_STORAGE_ROOT', ProductionStorage::DEFAULT_ROOT),
        storage_endpoint: ENV.fetch('ACTIVE_STORAGE_S3_ENDPOINT', nil),
        storage_bucket: ENV.fetch('ACTIVE_STORAGE_S3_BUCKET', nil),
        database_role: connection.select_value('SELECT current_user')
      )
    end

    def self.expected_targets_from_runtime
      ExpectedTargets.new(
        application_host: ENV.fetch('DEMO_RESET_EXPECTED_APPLICATION_HOST', nil),
        database_host: ENV.fetch('DEMO_RESET_EXPECTED_DATABASE_HOST', nil),
        storage_service: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_SERVICE', nil),
        storage_root: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_ROOT', nil),
        storage_endpoint: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_ENDPOINT', nil),
        storage_bucket: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_BUCKET', nil),
        database_role: ENV.fetch('DEMO_RESET_EXPECTED_DATABASE_ROLE', nil)
      )
    end

    def initialize(expected:, **targets)
      @targets = Targets.new(**targets)
      @expected = expected.is_a?(ExpectedTargets) ? expected : ExpectedTargets.new(**expected)
    end

    def call
      failed_targets = target_checks.reject { |_, valid| valid }.keys
      raise UnsafeTargetError, "demo reset refused: #{failed_targets.join(', ')}" if failed_targets.any?

      { outcome: 'passed', targets: target_checks.keys.map(&:to_s) }
    end

    private

    attr_reader :targets, :expected

    def target_checks
      application_checks.merge(storage_checks).merge(
        database_role: target_matches?(targets.database_role, expected.database_role)
      )
    end

    def application_checks
      {
        demo_mode: targets.demo_mode,
        application_host: target_matches?(application_host, expected.application_host),
        database_host: database_hosts_valid?
      }
    end

    def storage_checks
      { storage_service: storage_service_valid? }.merge(storage_backend_checks)
    end

    def storage_backend_checks
      disk_storage_checks.merge(s3_storage_checks)
    end

    def disk_storage_checks
      return {} unless disk_service?

      { storage_root: target_matches?(targets.storage_root, expected.storage_root) }
    end

    def s3_storage_checks
      return {} unless s3_service?

      {
        storage_endpoint: target_matches?(targets.storage_endpoint, expected.storage_endpoint),
        storage_bucket: target_matches?(targets.storage_bucket, expected.storage_bucket)
      }
    end

    def storage_service_valid?
      ProductionStorage::SERVICES.include?(storage_service) &&
        target_matches?(storage_service, expected.storage_service)
    end

    def disk_service?
      ProductionStorage::DISK_SERVICES.include?(storage_service)
    end

    def s3_service?
      ProductionStorage::S3_SERVICES.include?(storage_service)
    end

    def storage_service
      targets.storage_service.to_s
    end

    def application_host
      URI.parse(targets.application_url.to_s).host
    rescue URI::InvalidURIError
      nil
    end

    def database_hosts_valid?
      hosts = Array(targets.database_host)
      expected.database_host.present? && hosts.any? && hosts.all? { |host| host == expected.database_host }
    end

    def target_matches?(actual, expected_value)
      expected_value.present? && actual == expected_value
    end
  end
end
