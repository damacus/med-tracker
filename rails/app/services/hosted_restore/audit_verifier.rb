# frozen_string_literal: true

require 'json'

module HostedRestore
  class DefaultWormVerifierFactory
    def call(command_environment)
      Audit::Verification::WormVerifierFactory.new(command_environment).call
    end
  end

  class CombinedAuditCommand
    def initialize(environment: ENV,
                   worm_verifier_factory: DefaultWormVerifierFactory.new,
                   expected_head_evidence: nil)
      @environment = environment
      @worm_verifier_factory = worm_verifier_factory
      @expected_head_evidence = expected_head_evidence || ExpectedHeadEvidence.new
    end

    def call(household_id:, expected_head:)
      expected_head_evidence.call(household_id:, expected: expected_head)
      command_environment = environment_for(household_id)
      output, exit_code = execute(command_environment)
      return failure_result unless exit_code.zero?

      verified_result(output)
    rescue Audit::Verification::ConfigurationError, JSON::ParserError, KeyError
      failure_result
    end

    private

    attr_reader :environment, :worm_verifier_factory, :expected_head_evidence

    def environment_for(household_id)
      environment.to_h.merge(
        'HOUSEHOLD_ID' => household_id.to_s,
        'SCOPE' => 'combined',
        'FORMAT' => 'json'
      )
    end

    def execute(command_environment)
      output = StringIO.new
      error_output = StringIO.new
      worm_verifier = worm_verifier_factory.call(command_environment)
      exit_code = Audit::Verification::Command.new(
        environment: command_environment, output:, error_output:, worm_verifier:
      ).call
      [output, exit_code]
    end

    def verified_result(output)
      result = JSON.parse(output.string, symbolize_names: true)
      parsed = {
        checked_entries: result.fetch(:checked_entries), checked_checkpoints: result.fetch(:checked_checkpoints),
        checked_objects: result.fetch(:checked_objects), issue_codes: result.fetch(:issues).pluck(:code)
      }
      return parsed.merge(verified_head: false) if parsed[:checked_checkpoints].to_i < 1 ||
                                                   parsed[:checked_objects].to_i < 2

      parsed.merge(verified_head: true)
    end

    def failure_result
      {
        checked_entries: 0, checked_checkpoints: 0, checked_objects: 0,
        issue_codes: ['verification_failed'], verified_head: false
      }
    end
  end

  class ExpectedHeadEvidence
    def call(household_id:, expected:)
      checkpoint = AuditCheckpoint.find_by(
        household_id:, chain_epoch: expected.fetch('chain_epoch'), sequence: expected.fetch('sequence'),
        entry_hash: [expected.fetch('entry_hash')].pack('H*')
      )
      ledger_entry = AuditLedgerEntry.find_by(
        household_id:, chain_epoch: expected.fetch('chain_epoch'), sequence: expected.fetch('sequence'),
        entry_hash: [expected.fetch('entry_hash')].pack('H*')
      )
      valid = checkpoint&.signature.present? && ledger_entry && delivered?(checkpoint:, ledger_entry:)
      raise VerificationError, 'expected_head_evidence_missing' unless valid

      true
    rescue ArgumentError
      raise VerificationError, 'expected_head_evidence_missing'
    end

    private

    def delivered?(checkpoint:, ledger_entry:)
      AuditExportDelivery.exists?(audit_checkpoint: checkpoint, status: 'delivered') &&
        AuditExportDelivery.exists?(audit_ledger_entry: ledger_entry, status: 'delivered')
    end
  end

  class AuditVerifier
    def initialize(household_ids:, expected_heads:, command: CombinedAuditCommand.new,
                   connection: ActiveRecord::Base.connection, current_role: nil)
      @household_ids = household_ids.map { |value| canonical_integer(value) }
      @expected_heads = expected_heads
      @command = command
      @connection = connection
      @current_role = current_role || -> { connection.select_value('SELECT current_user') }
    end

    def call
      verify_households!
      verify_expected_heads!
      raise VerificationError, 'audit_verifier_role_required' unless current_role.call == 'med_tracker_audit_verifier'

      summarize(verify_samples)
    rescue KeyError
      raise VerificationError, 'worm_heads_invalid'
    end

    private

    attr_reader :household_ids, :expected_heads, :command, :connection, :current_role

    def verify_households!
      return if household_ids.size == 2 && household_ids.all?(&:positive?) && household_ids.uniq.size == 2

      raise VerificationError, 'distinct_household_samples_required'
    end

    def verify_expected_heads!
      valid = expected_heads.is_a?(Hash) && expected_heads.keys.sort == %w[sample_a sample_b] &&
              expected_heads.values.all? { |head| valid_expected_head?(head) }
      raise VerificationError, 'worm_heads_invalid' unless valid
    end

    def valid_expected_head?(head)
      expected_head_shape?(head) && valid_epoch?(head) && valid_sequence?(head) && valid_entry_hash?(head)
    end

    def expected_head_shape?(head)
      head.is_a?(Hash) && head.keys.sort == %w[chain_epoch entry_hash sequence]
    end

    def valid_epoch?(head)
      value = head['chain_epoch']
      value.is_a?(String) && value.match?(/\A[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\z/)
    end

    def valid_sequence?(head)
      value = head['sequence']
      value.is_a?(Integer) && value.between?(1, (2**63) - 1)
    end

    def valid_entry_hash?(head)
      value = head['entry_hash']
      value.is_a?(String) && value.match?(/\A[a-f0-9]{64}\z/)
    end

    def verify_samples
      %w[sample_a sample_b].each_with_index.map do |label, index|
        verify_sample(household_ids.fetch(index), expected_heads.fetch(label))
      end
    end

    def summarize(results)
      {
        scope: 'combined', samples_verified: results.size,
        checked_entries: results.sum { |result| result.fetch(:checked_entries) },
        checked_checkpoints: results.sum { |result| result.fetch(:checked_checkpoints) },
        checked_objects: results.sum { |result| result.fetch(:checked_objects) },
        verified_heads: results.count { |result| result.fetch(:verified_head) },
        worm_comparison: 'match'
      }
    end

    def verify_sample(household_id, expected_head)
      result = with_household(household_id) { command.call(household_id:, expected_head:) }
      raise VerificationError, 'audit_worm_verification_failed' if result.fetch(:issue_codes).any?
      raise VerificationError, 'worm_restore_divergence' unless result.fetch(:verified_head)

      result
    end

    def with_household(household_id)
      connection.transaction(requires_new: true) do
        apply_household_setting(household_id)
        yield
      ensure
        apply_household_setting('')
      end
    end

    def apply_household_setting(value)
      connection.execute(
        ActiveRecord::Base.sanitize_sql_array(
          ['SELECT set_config(?, ?, true)', TenantContext::SETTING_NAMES.fetch(:household), value.to_s]
        )
      )
    end

    def canonical_integer(value)
      string = value.to_s
      unless string.match?(/\A[1-9]\d*\z/) && Integer(string, 10) <= (2**63) - 1
        raise VerificationError, 'household_id_invalid'
      end

      Integer(string, 10)
    end
  end
end
