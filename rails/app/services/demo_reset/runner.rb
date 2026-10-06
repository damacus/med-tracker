# frozen_string_literal: true

module DemoReset
  class Runner
    def initialize(**dependencies)
      @preflight = dependencies.fetch(:preflight) { Preflight.from_runtime }
      @primary_reset = dependencies.fetch(:primary_reset) { PrimaryDatabaseReset.new }
      @auxiliary_reset = dependencies.fetch(:auxiliary_reset) { AuxiliaryDatabasesReset.new }
      @storage_cleaner = dependencies.fetch(:storage_cleaner) { StorageCleaner.new }
      @verifier = dependencies.fetch(:verifier) { Verifier.new }
      @reporter = dependencies.fetch(:reporter) { Reporter.new }
    end

    def call
      run_stage('preflight') { preflight.call }
      primary = run_stage('primary') { primary_reset.call }
      auxiliary = run_stage('auxiliary') { auxiliary_reset.call }
      storage = run_stage('storage_cleaner') { storage_cleaner.call }
      verification = run_stage('verification') { verifier.call }

      {
        outcome: 'succeeded',
        baseline: DemoBaseline::IDENTIFIER,
        primary:,
        auxiliary:,
        storage:,
        verification:
      }
    end

    private

    attr_reader :preflight, :primary_reset, :auxiliary_reset, :storage_cleaner, :verifier, :reporter

    def run_stage(name)
      result = yield
      reporter.stage(name, 'succeeded')
      result
    rescue StandardError
      reporter.stage(name, 'failed')
      raise
    end
  end
end
