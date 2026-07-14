# frozen_string_literal: true

require 'fileutils'
require 'json'
require 'open3'
require 'securerandom'
require 'time'

module HostedRestore
  CommandResult = Data.define(:exit_code, :output)

  class Input
    class Invalid < StandardError; end

    REQUIRED = %w[
      DATABASE_BACKUP_ID ATTACHMENT_BACKUP_ID RESTORE_TARGET_ID APP_IMAGE TESTER
      HOUSEHOLD_A_ID HOUSEHOLD_B_ID WORM_REFERENCE WORM_HEADS_JSON EVIDENCE_OUTPUT
    ].freeze
    PLACEHOLDERS = %w[current latest todo tbd unknown].freeze

    attr_reader :values

    def initialize(environment)
      @values = {}
      REQUIRED.each { |name| @values[name] = environment[name].to_s.strip }
      validate!
    end

    def [](name)
      values.fetch(name)
    end

    def household_ids
      [self['HOUSEHOLD_A_ID'].to_i, self['HOUSEHOLD_B_ID'].to_i]
    end

    def worm_heads
      @worm_heads ||= JSON.parse(self['WORM_HEADS_JSON'])
    rescue JSON::ParserError
      raise Invalid, 'worm_heads_invalid'
    end

    private

    def validate!
      validate_presence!
      validate_content!
      validate_identifiers!
      validate_worm_heads!
      validate_output!
    end

    def validate_presence!
      raise Invalid, 'required_input_missing' if values.value?('')
    end

    def validate_content!
      raise Invalid, 'control_character_not_allowed' if values.values.any? { |value| value.match?(/[[:cntrl:]]/) }
      raise Invalid, 'placeholder_not_allowed' if placeholder_value?
    end

    def validate_identifiers!
      raise Invalid, 'opaque_identifier_required' unless opaque_identifiers?
      raise Invalid, 'immutable_app_image_required' unless immutable_image?
      raise Invalid, 'distinct_household_samples_required' unless valid_household_ids?
    end

    def placeholder_value?
      values.except('WORM_HEADS_JSON', 'EVIDENCE_OUTPUT').values.any? do |value|
        PLACEHOLDERS.include?(value.downcase)
      end
    end

    def immutable_image?
      image = self['APP_IMAGE']
      return false if image.end_with?(':latest')

      image.match?(%r{\A[a-zA-Z0-9][a-zA-Z0-9._/-]*(?::[a-zA-Z0-9][a-zA-Z0-9._-]*|@sha256:[a-f0-9]{64})\z})
    end

    def opaque_identifiers?
      %w[DATABASE_BACKUP_ID ATTACHMENT_BACKUP_ID RESTORE_TARGET_ID TESTER WORM_REFERENCE].all? do |name|
        self[name].match?(%r{\A[a-zA-Z0-9][a-zA-Z0-9._:@/-]*\z})
      end
    end

    def valid_household_ids?
      household_ids.all?(&:positive?) && household_ids.uniq.size == 2
    end

    def validate_worm_heads!
      %w[sample_a sample_b].each do |label|
        head = worm_heads.fetch(label) { raise Invalid, 'worm_heads_invalid' }
        valid = head['chain_epoch'].to_s.match?(/\A[0-9a-f-]{36}\z/) && head['sequence'].to_i.positive? &&
                head['entry_hash'].to_s.match?(/\A[a-f0-9]{64}\z/)
        raise Invalid, 'worm_heads_invalid' unless valid
      end
    end

    def validate_output!
      output = Pathname.new(self['EVIDENCE_OUTPUT'])
      raise Invalid, 'evidence_output_must_be_absolute' unless output.absolute?
      raise Invalid, 'evidence_parent_missing' unless output.parent.directory?
    end
  end

  class EvidenceWriter
    class AlreadyExists < StandardError; end

    def initialize(output)
      @output = Pathname.new(output)
    end

    def ensure_available!
      raise AlreadyExists, 'evidence_output_exists' if output.exist?

      Dir.mkdir(output, 0o700)
    rescue Errno::EEXIST
      raise AlreadyExists, 'evidence_output_exists'
    end

    def write(evidence)
      write_exclusive(output.join('evidence.json'), "#{JSON.pretty_generate(evidence)}\n")
      write_exclusive(output.join('evidence.md'), markdown(evidence))
    rescue Errno::EEXIST
      raise AlreadyExists, 'evidence_output_exists'
    end

    private

    attr_reader :output

    def write_exclusive(path, content)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(content) }
    end

    def markdown(evidence)
      <<~MARKDOWN
        # Hosted restore rehearsal evidence

        #{metadata_markdown(evidence)}

        ## Commands

        #{commands_markdown(evidence)}

        ## Failures and remediation

        #{failures_markdown(evidence)}

        Final outcome: #{evidence.fetch(:outcome).upcase}
      MARKDOWN
    end

    def commands_markdown(evidence)
      evidence.fetch(:commands).map do |command|
        "- `#{command.fetch(:id)}`: #{command.fetch(:status).upcase}"
      end.join("\n")
    end

    def failures_markdown(evidence)
      failures = evidence.fetch(:failures).map do |failure|
        "- `#{failure.fetch(:command_id)}`: #{failure.fetch(:remediation)}"
      end.join("\n")
      failures.empty? ? '- None' : failures
    end

    def metadata_markdown(evidence)
      <<~MARKDOWN
        - Performed at: #{evidence.fetch(:performed_at)}
        - Database backup: `#{evidence.dig(:inputs, :database_backup_id)}`
        - Attachment backup: `#{evidence.dig(:inputs, :attachment_backup_id)}`
        - Restore target: `#{evidence.dig(:inputs, :restore_target_id)}`
        - Application image: `#{evidence.dig(:inputs, :app_image)}`
        - Schema version: `#{evidence.fetch(:schema_version, 'unavailable')}`
        - Tester: `#{evidence.dig(:inputs, :tester)}`
        - WORM reference: `#{evidence.dig(:inputs, :worm_reference)}`
      MARKDOWN
    end
  end

  class CommandRunner
    def call(command)
      stdout, _stderr, status = Open3.capture3(*command.fetch(:argv))
      CommandResult.new(exit_code: status.exitstatus || 1, output: parsed_output(command.fetch(:id), stdout))
    rescue SystemCallError
      CommandResult.new(exit_code: 1, output: { outcome: 'failed', failure_code: 'command_unavailable' })
    end

    private

    def parsed_output(command_id, stdout)
      return { outcome: 'passed' } if command_id == 'owner.db_migrate'

      line = stdout.lines.rfind { |candidate| candidate.lstrip.start_with?('{') }
      JSON.parse(line, symbolize_names: true)
    rescue JSON::ParserError, TypeError
      { outcome: 'failed', failure_code: 'invalid_command_output' }
    end
  end

  class Rehearsal
    COMMANDS = [
      {
        id: 'owner.db_migrate',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=migrate-prod',
               'COMMAND=env DATABASE_ROLE=med_tracker_owner rails db:migrate']
      },
      {
        id: 'runtime.restore_verify',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=web-prod',
               'DOCKER_RUN_ARGS=-e HOUSEHOLD_A_ID -e HOUSEHOLD_B_ID',
               'COMMAND=env DATABASE_ROLE=med_tracker_app rails hosted_restore:verify_runtime']
      },
      {
        id: 'audit.combined_verify',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=audit-verifier-prod',
               'DOCKER_RUN_ARGS=-e HOUSEHOLD_A_ID -e HOUSEHOLD_B_ID -e WORM_HEADS_JSON',
               'COMMAND=rails hosted_restore:verify_audit']
      }
    ].freeze
    OUTPUT_KEYS = %i[
      outcome failure_code schema_version database_role forced_rls default_deny isolation storage scope
      samples_verified checked_entries checked_checkpoints checked_objects worm_comparison
    ].freeze

    def initialize(environment: ENV, runner: CommandRunner.new, clock: -> { Time.now.utc })
      @input = Input.new(environment)
      @runner = runner
      @clock = clock
      @writer = EvidenceWriter.new(input['EVIDENCE_OUTPUT'])
    end

    def call
      writer.ensure_available!
      command_evidence, failures = execute_commands
      evidence = build_evidence(command_evidence, failures)
      writer.write(evidence)
      evidence.fetch(:outcome) == 'passed' ? 0 : 1
    end

    private

    attr_reader :input, :runner, :clock, :writer

    def execute_commands
      command_evidence = []
      failures = []

      COMMANDS.each do |command|
        result = runner.call(command)
        command_evidence << command_record(command, result)
        next if result.exit_code.zero? && result.output[:outcome].to_s != 'failed'

        failures << failure_record(command, result)
        break
      end

      [command_evidence, failures]
    end

    def command_record(command, result)
      passed = result.exit_code.zero? && result.output[:outcome].to_s != 'failed'
      {
        id: command.fetch(:id), status: passed ? 'passed' : 'failed',
        exit_code: result.exit_code, output: result.output.slice(*OUTPUT_KEYS)
      }
    end

    def failure_record(command, result)
      {
        command_id: command.fetch(:id),
        failure_code: result.output.fetch(:failure_code, 'stage_failed'),
        remediation: 'correct the isolated restore or configuration and repeat the same rehearsal'
      }
    end

    def build_evidence(commands, failures)
      runtime = commands.find { |command| command[:id] == 'runtime.restore_verify' }
      {
        evidence_schema_version: 1, rehearsal_id: SecureRandom.uuid, performed_at: clock.call.iso8601,
        inputs: evidence_inputs, schema_version: runtime&.dig(:output, :schema_version), commands:,
        failures:, audit_worm_comparison: audit_summary(commands),
        outcome: failures.empty? && commands.size == COMMANDS.size ? 'passed' : 'failed'
      }.compact
    end

    def evidence_inputs
      {
        database_backup_id: input['DATABASE_BACKUP_ID'], attachment_backup_id: input['ATTACHMENT_BACKUP_ID'],
        restore_target_id: input['RESTORE_TARGET_ID'], app_image: input['APP_IMAGE'], tester: input['TESTER'],
        worm_reference: input['WORM_REFERENCE']
      }
    end

    def audit_summary(commands)
      commands.find { |command| command[:id] == 'audit.combined_verify' }&.fetch(:output)
    end
  end
end
