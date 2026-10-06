# frozen_string_literal: true

module DemoReset
  class Verifier
    def initialize(baseline_verifier: DemoBaseline::Loader.new.method(:verify!),
                   auxiliary_verifier: AuxiliaryDatabasesReset.new.method(:verify_empty!),
                   storage_empty: StorageCleaner.new.method(:empty?), demo_mode: DemoMode.method(:enabled?))
      @baseline_verifier = baseline_verifier
      @auxiliary_verifier = auxiliary_verifier
      @storage_empty = storage_empty
      @demo_mode = demo_mode
    end

    def call
      raise VerificationError, 'demo_mode_disabled' unless demo_mode.call

      baseline = baseline_verifier.call
      auxiliary = auxiliary_verifier.call
      verify_storage!

      baseline.merge(auxiliary_databases: auxiliary, storage_empty: true)
    rescue StorageCleanupError
      raise VerificationError, 'storage_unavailable'
    end

    private

    attr_reader :baseline_verifier, :auxiliary_verifier, :storage_empty, :demo_mode

    def verify_storage!
      raise VerificationError, 'storage_not_empty' unless storage_empty.call
    end
  end
end
