# frozen_string_literal: true

module DemoReset
  class Verifier
    def initialize(baseline_verifier: DemoBaseline::Loader.new.method(:verify!),
                   auxiliary_verifier: AuxiliaryDatabasesReset.new.method(:verify_empty!),
                   storage_root: ENV.fetch('ACTIVE_STORAGE_ROOT', nil),
                   health_checker: HealthChecker.new.method(:call), demo_mode: DemoMode.method(:enabled?))
      @baseline_verifier = baseline_verifier
      @auxiliary_verifier = auxiliary_verifier
      @storage_root = Pathname(storage_root.to_s)
      @health_checker = health_checker
      @demo_mode = demo_mode
    end

    def call
      raise VerificationError, 'demo_mode_disabled' unless demo_mode.call

      baseline = baseline_verifier.call
      auxiliary = auxiliary_verifier.call
      storage_entries = verify_storage!
      verify_health!

      baseline.merge(auxiliary_databases: auxiliary, storage_entries:, health: 'available')
    rescue Errno::ENOENT, Errno::EACCES
      raise VerificationError, 'storage_unavailable'
    end

    private

    attr_reader :baseline_verifier, :auxiliary_verifier, :storage_root, :health_checker, :demo_mode

    def verify_storage!
      storage_root.children.size.tap do |storage_entries|
        raise VerificationError, 'storage_not_empty' if storage_entries.positive?
      end
    end

    def verify_health!
      raise VerificationError, 'application_unavailable' unless health_checker.call
    end
  end
end
