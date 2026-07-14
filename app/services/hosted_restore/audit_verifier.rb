# frozen_string_literal: true

require 'json'

module HostedRestore
  class CombinedAuditCommand
    def initialize(environment: ENV)
      @environment = environment
    end

    def call(household_id:)
      command_environment = environment_for(household_id)
      output, exit_code = execute(command_environment)
      return failure_result unless exit_code.zero?

      parsed_result(output)
    rescue Audit::Verification::ConfigurationError, JSON::ParserError, KeyError
      failure_result
    end

    private

    attr_reader :environment

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
      worm_verifier = Audit::Verification::WormVerifierFactory.new(command_environment).call
      exit_code = Audit::Verification::Command.new(
        environment: command_environment, output:, error_output:, worm_verifier:
      ).call
      [output, exit_code]
    end

    def parsed_result(output)
      result = JSON.parse(output.string, symbolize_names: true)
      {
        checked_entries: result.fetch(:checked_entries), checked_checkpoints: result.fetch(:checked_checkpoints),
        checked_objects: result.fetch(:checked_objects), issue_codes: result.fetch(:issues).pluck(:code)
      }
    end

    def failure_result
      { checked_entries: 0, checked_checkpoints: 0, checked_objects: 0, issue_codes: ['verification_failed'] }
    end
  end

  class WormHeadComparator
    def call(household_id:, expected:)
      checkpoint = AuditCheckpoint.find_by(
        household_id:, chain_epoch: expected.fetch('chain_epoch'), sequence: expected.fetch('sequence')
      )
      return false unless checkpoint&.signature.present? && checkpoint.audit_export_delivery&.delivered?

      ActiveSupport::SecurityUtils.secure_compare(
        checkpoint.entry_hash.unpack1('H*'), expected.fetch('entry_hash')
      )
    end
  end

  class AuditVerifier
    def initialize(household_ids:, expected_heads:, command: CombinedAuditCommand.new,
                   head_comparator: WormHeadComparator.new,
                   current_role: -> { ActiveRecord::Base.connection.select_value('SELECT current_user') })
      @household_ids = household_ids.map(&:to_i)
      @expected_heads = expected_heads
      @command = command
      @head_comparator = head_comparator
      @current_role = current_role
    end

    def call
      verify_households!
      raise VerificationError, 'audit_verifier_role_required' unless current_role.call == 'med_tracker_audit_verifier'

      summarize(verify_samples)
    rescue KeyError
      raise VerificationError, 'worm_heads_invalid'
    end

    private

    attr_reader :household_ids, :expected_heads, :command, :head_comparator, :current_role

    def verify_households!
      return if household_ids.size == 2 && household_ids.all?(&:positive?) && household_ids.uniq.size == 2

      raise VerificationError, 'distinct_household_samples_required'
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
        worm_comparison: 'match'
      }
    end

    def verify_sample(household_id, expected_head)
      result = command.call(household_id:)
      raise VerificationError, 'audit_worm_verification_failed' if result.fetch(:issue_codes).any?
      unless head_comparator.call(household_id:, expected: expected_head)
        raise VerificationError, 'worm_restore_divergence'
      end

      result
    end
  end
end
