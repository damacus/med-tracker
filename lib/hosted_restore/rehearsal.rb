# frozen_string_literal: true

require 'fileutils'
require 'digest'
require 'json'
require 'open3'
require 'securerandom'
require 'time'
require 'tmpdir'

module HostedRestore
  CommandResult = Data.define(:exit_code, :output)

  class Input
    class Invalid < StandardError; end

    REQUIRED = %w[
      DATABASE_BACKUP_ID ATTACHMENT_BACKUP_ID RESTORE_TARGET_ID APP_IMAGE TESTER
      HOUSEHOLD_A_ID HOUSEHOLD_B_ID WORM_REFERENCE WORM_HEADS_JSON EVIDENCE_ROOT EVIDENCE_OUTPUT
    ].freeze
    PLACEHOLDERS = %w[current latest todo tbd unknown].freeze
    UUID_PATTERN = /\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/
    INTEGER_PATTERN = /\A[1-9]\d*\z/
    MAX_BIGINT = (2**63) - 1

    attr_reader :values, :evidence_root, :evidence_output

    def initialize(environment, repository_root: File.expand_path('../..', __dir__), temporary_roots: nil)
      @values = {}
      REQUIRED.each { |name| @values[name] = environment[name].to_s.strip }
      @repository_root = canonical_reference_path(repository_root)
      @temporary_roots = temporary_roots&.map { |path| canonical_reference_path(path) }
      validate!
    end

    def [](name)
      values.fetch(name)
    end

    def household_ids
      [canonical_integer('HOUSEHOLD_A_ID'), canonical_integer('HOUSEHOLD_B_ID')]
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
      raise Invalid, 'household_id_invalid' unless valid_household_id_inputs?
      raise Invalid, 'distinct_household_samples_required' unless valid_household_ids?
    end

    def placeholder_value?
      values.except('WORM_HEADS_JSON', 'EVIDENCE_OUTPUT').values.any? do |value|
        PLACEHOLDERS.include?(value.downcase)
      end
    end

    def immutable_image?
      immutable_image_reference?(self['APP_IMAGE'])
    end

    def immutable_image_reference?(image)
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

    def valid_household_id_inputs?
      %w[HOUSEHOLD_A_ID HOUSEHOLD_B_ID].all? { |name| canonical_integer?(self[name]) }
    end

    def validate_worm_heads!
      raise Invalid, 'worm_heads_invalid' unless valid_worm_heads?
    end

    def valid_worm_heads?
      worm_heads.is_a?(Hash) && worm_heads.keys.sort == %w[sample_a sample_b] &&
        worm_heads.values.all? { |head| valid_worm_head?(head) }
    end

    def valid_worm_head?(head)
      worm_head_shape?(head) && valid_chain_epoch?(head) && valid_sequence?(head) && valid_entry_hash?(head)
    end

    def worm_head_shape?(head)
      head.is_a?(Hash) && head.keys.sort == %w[chain_epoch entry_hash sequence]
    end

    def valid_chain_epoch?(head)
      value = head['chain_epoch']
      value.is_a?(String) && value.match?(UUID_PATTERN)
    end

    def valid_sequence?(head)
      value = head['sequence']
      value.is_a?(Integer) && value.between?(1, MAX_BIGINT)
    end

    def valid_entry_hash?(head)
      value = head['entry_hash']
      value.is_a?(String) && value.match?(/\A[a-f0-9]{64}\z/)
    end

    def validate_output!
      @evidence_root = canonical_existing_path(self['EVIDENCE_ROOT'], 'evidence_root_invalid')
      reject_unsafe_root!
      @evidence_output = canonical_output_path
    end

    def canonical_output_path
      raw_output = Pathname.new(self['EVIDENCE_OUTPUT'])
      raise Invalid, 'evidence_output_must_be_absolute' unless raw_output.absolute?

      output = raw_output.expand_path
      parent = canonical_existing_path(output.parent, 'evidence_parent_missing')
      raise Invalid, 'evidence_output_outside_root' unless contained?(parent, evidence_root)

      parent.join(output.basename)
    end

    def canonical_integer(name)
      Integer(self[name], 10)
    end

    def canonical_integer?(value)
      value.match?(INTEGER_PATTERN) && Integer(value, 10) <= MAX_BIGINT
    end

    def canonical_existing_path(path, failure_code)
      expanded = Pathname.new(path).expand_path
      raise Invalid, failure_code unless expanded.directory?

      Pathname.new(File.realpath(expanded))
    rescue Errno::ENOENT, Errno::EACCES
      raise Invalid, failure_code
    end

    def reject_unsafe_root!
      forbidden = resolved_temporary_roots + [repository_root]
      unsafe = evidence_root.root? || forbidden.any? { |path| contained?(evidence_root, path) }
      raise Invalid, 'evidence_root_not_durable' if unsafe
    end

    def resolved_temporary_roots
      return temporary_roots if temporary_roots

      [Dir.tmpdir, '/tmp', '/private/tmp', '/var/tmp'].uniq.filter_map do |path|
        candidate = Pathname.new(path)
        candidate.realpath if candidate.exist?
      end
    end

    def canonical_reference_path(path)
      candidate = Pathname.new(path).expand_path
      candidate.exist? ? candidate.realpath : candidate
    end

    def contained?(candidate, root)
      candidate == root || candidate.to_s.start_with?("#{root}#{File::SEPARATOR}")
    end

    attr_reader :repository_root, :temporary_roots
  end

  class AtomicFileWriter
    def call(path, content)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
        file.write(content)
        file.fsync
      end
    end
  end

  class EvidenceWriter
    class AlreadyExists < StandardError; end

    def initialize(root:, output:, file_writer: AtomicFileWriter.new, token: -> { SecureRandom.hex(12) })
      @root = Pathname.new(root)
      @output = Pathname.new(output)
      @file_writer = file_writer
      @staging = @output.parent.join(".#{@output.basename}.#{token.call}.tmp")
      @lock_path = @output.parent.join(".#{@output.basename}.lock")
    end

    def ensure_available!
      ensure_contained!
      raise AlreadyExists, 'evidence_output_exists' if output.exist?

      reserve_bundle!
    rescue Errno::EEXIST
      raise AlreadyExists, 'evidence_output_exists'
    end

    def write(evidence)
      ensure_contained!
      write_staged_bundle(evidence)
      publish_bundle!
    rescue Errno::EEXIST
      cleanup_reservation!
      raise AlreadyExists, 'evidence_output_exists'
    rescue StandardError
      cleanup_reservation!
      raise
    end

    private

    attr_reader :root, :output, :file_writer, :staging, :lock_path, :lock_file

    def ensure_contained!
      canonical_root = Pathname.new(File.realpath(root))
      return if contained_in_root?(canonical_output_candidate, canonical_root)

      raise AlreadyExists, 'evidence_output_outside_root'
    rescue Errno::ENOENT, Errno::EACCES
      raise AlreadyExists, 'evidence_path_unavailable'
    end

    def canonical_output_candidate
      return Pathname.new(File.realpath(output)) if output.exist?

      Pathname.new(File.realpath(output.parent)).join(output.basename)
    end

    def contained_in_root?(candidate, canonical_root)
      candidate.to_s.start_with?("#{canonical_root}#{File::SEPARATOR}")
    end

    def reserve_bundle!
      @lock_file = File.open(lock_path, File::WRONLY | File::CREAT | File::EXCL, 0o600)
      Dir.mkdir(staging, 0o700)
    rescue StandardError
      cleanup_reservation!
      raise
    end

    def write_staged_bundle(evidence)
      write_evidence_files(evidence)
      write_completion_marker(evidence)
      fsync_directory(staging)
    end

    def write_evidence_files(evidence)
      file_writer.call(staging.join('evidence.json'), "#{JSON.pretty_generate(evidence)}\n")
      file_writer.call(staging.join('evidence.md'), markdown(evidence))
    end

    def write_completion_marker(evidence)
      file_writer.call(staging.join('complete.json'), "#{JSON.pretty_generate(completion_manifest(evidence))}\n")
    end

    def publish_bundle!
      File.rename(staging, output)
      fsync_directory(output.parent)
      release_lock!
    end

    def completion_manifest(evidence)
      {
        outcome: evidence.fetch(:outcome),
        files: {
          'evidence.json' => Digest::SHA256.file(staging.join('evidence.json')).hexdigest,
          'evidence.md' => Digest::SHA256.file(staging.join('evidence.md')).hexdigest
        }
      }
    end

    def fsync_directory(path)
      File.open(path, File::RDONLY, &:fsync)
    end

    def cleanup_reservation!
      FileUtils.rm_rf(staging)
      release_lock!
    end

    def release_lock!
      return unless lock_file

      lock_file.close unless lock_file.closed?
      FileUtils.rm_f(lock_path)
      @lock_file = nil
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
        [
          "- `#{command.fetch(:description)}`: #{command.fetch(:status).upcase}",
          output_markdown(command.fetch(:output))
        ].join("\n")
      end.join("\n")
    end

    def output_markdown(output, prefix = nil)
      output.flat_map do |key, value|
        label = [prefix, key].compact.join('.')
        value.is_a?(Hash) ? output_markdown(value, label) : "  - `#{label}`: `#{value}`"
      end.join("\n")
    end

    def failures_markdown(evidence)
      failures = evidence.fetch(:failures).map do |failure|
        "- `#{failure.fetch(:command_id)}` (`#{failure.fetch(:failure_code)}`): #{failure.fetch(:remediation)}"
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
    SAFE_KEYS = %i[
      outcome failure_code schema_version database_role app_image forced_rls default_deny isolation storage scope
      samples_verified checked_entries checked_checkpoints checked_objects verified_heads worm_comparison
    ].freeze

    def initialize(capture: Open3.method(:capture3))
      @capture = capture
    end

    def call(command)
      stdout, stderr, status = capture.call(*command.fetch(:argv))
      output = parsed_output(status.exitstatus&.zero? ? stdout : stderr)
      CommandResult.new(exit_code: status.exitstatus || 1, output:)
    rescue SystemCallError
      CommandResult.new(exit_code: 1, output: { outcome: 'failed', failure_code: 'command_unavailable' })
    end

    private

    attr_reader :capture

    def parsed_output(stream)
      line = stream.lines.rfind { |candidate| candidate.lstrip.start_with?('{') }
      parsed = JSON.parse(line, symbolize_names: true)
      parsed.is_a?(Hash) ? parsed.slice(*SAFE_KEYS) : invalid_output
    rescue JSON::ParserError, TypeError
      invalid_output
    end

    def invalid_output
      { outcome: 'failed', failure_code: 'invalid_command_output' }
    end
  end

  class Rehearsal
    COMMANDS = [
      {
        id: 'owner.db_migrate',
        description: 'env DATABASE_ROLE=med_tracker_owner rails hosted_restore:migrate',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=migrate-prod',
               'COMMAND=env DATABASE_ROLE=med_tracker_owner rails hosted_restore:migrate']
      },
      {
        id: 'runtime.restore_verify',
        description: 'env DATABASE_ROLE=med_tracker_app rails hosted_restore:verify_runtime',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=web-prod',
               'DOCKER_RUN_ARGS=-e HOUSEHOLD_A_ID -e HOUSEHOLD_B_ID',
               'COMMAND=env DATABASE_ROLE=med_tracker_app rails hosted_restore:verify_runtime']
      },
      {
        id: 'audit.combined_verify',
        description: 'env DATABASE_ROLE=med_tracker_audit_verifier rails hosted_restore:verify_audit',
        argv: ['task', 'internal:run', 'ENVIRONMENT=prod', 'SERVICE=audit-verifier-prod',
               'DOCKER_RUN_ARGS=-e HOUSEHOLD_A_ID -e HOUSEHOLD_B_ID -e WORM_HEADS_JSON',
               'COMMAND=rails hosted_restore:verify_audit']
      }
    ].freeze
    SCHEMAS = {
      'owner.db_migrate' => %i[outcome schema_version database_role],
      'runtime.restore_verify' => %i[
        outcome schema_version database_role app_image forced_rls default_deny isolation storage
      ],
      'audit.combined_verify' => %i[
        outcome scope samples_verified checked_entries checked_checkpoints checked_objects verified_heads
        worm_comparison
      ]
    }.freeze

    def initialize(environment: ENV, runner: CommandRunner.new, clock: -> { Time.now.utc },
                   repository_root: File.expand_path('../..', __dir__), temporary_roots: nil)
      @input = Input.new(environment, repository_root:, temporary_roots:)
      @runner = runner
      @clock = clock
      @writer = EvidenceWriter.new(root: input.evidence_root, output: input.evidence_output)
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
        valid = valid_stage_result?(command, result, command_evidence)
        command_evidence << command_record(command, result, valid:)
        next if valid

        failures << failure_record(command, result, valid:)
        break
      end

      [command_evidence, failures]
    end

    def command_record(command, result, valid:)
      {
        id: command.fetch(:id), description: command.fetch(:description), status: valid ? 'passed' : 'failed',
        exit_code: result.exit_code, output: sanitized_output(command, result.output, valid:)
      }
    end

    def failure_record(command, result, valid:)
      {
        command_id: command.fetch(:id),
        failure_code: stage_failure_code(result, valid:),
        remediation: 'correct the isolated restore or configuration and repeat the same rehearsal'
      }
    end

    def stage_failure_code(result, valid:)
      return 'invalid_stage_output' if valid || result.output[:outcome] != 'failed'

      safe_failure_code(result.output)
    end

    def valid_stage_result?(command, result, previous_commands)
      return false unless result.exit_code.zero? && result.output[:outcome] == 'passed'
      return false unless result.output.keys.sort == SCHEMAS.fetch(command.fetch(:id)).sort

      valid_stage_payload?(command.fetch(:id), result.output, previous_commands)
    end

    def valid_stage_payload?(command_id, output, previous_commands)
      case command_id
      when 'owner.db_migrate' then valid_owner_output?(output)
      when 'runtime.restore_verify' then valid_runtime_output?(output, previous_commands)
      when 'audit.combined_verify' then valid_audit_output?(output)
      end
    end

    def valid_owner_output?(output)
      output[:database_role] == 'med_tracker_owner' && schema_version?(output[:schema_version])
    end

    def valid_runtime_output?(output, previous_commands)
      owner_version = previous_commands.first&.dig(:output, :schema_version)
      valid_runtime_identity?(output, owner_version) &&
        valid_runtime_controls?(output) &&
        valid_runtime_samples?(output)
    end

    def valid_runtime_identity?(output, owner_version)
      output[:database_role] == 'med_tracker_app' && output[:app_image] == input['APP_IMAGE'] &&
        output[:schema_version] == owner_version
    end

    def valid_runtime_controls?(output)
      output[:forced_rls] == true && output[:default_deny] == true &&
        output[:isolation] == { clinical: true, audit: true, attachments: true }
    end

    def valid_runtime_samples?(output)
      output[:storage] == { samples_verified: 2 }
    end

    def valid_audit_output?(output)
      output[:scope] == 'combined' && output[:samples_verified] == 2 && output[:verified_heads] == 2 &&
        positive_integer?(output[:checked_entries]) && integer_at_least?(output[:checked_checkpoints], 2) &&
        integer_at_least?(output[:checked_objects], 4) && output[:worm_comparison] == 'match'
    end

    def schema_version?(value)
      value.is_a?(String) && value.match?(/\A\d{14}\z/)
    end

    def positive_integer?(value)
      value.is_a?(Integer) && value.positive?
    end

    def integer_at_least?(value, minimum)
      value.is_a?(Integer) && value >= minimum
    end

    def valid_failure_code?(value)
      value.is_a?(String) && value.match?(/\A[a-z0-9_]+\z/)
    end

    def sanitized_output(command, output, valid:)
      return { outcome: 'failed', failure_code: safe_failure_code(output) } unless valid

      output.slice(*SCHEMAS.fetch(command.fetch(:id)))
    end

    def safe_failure_code(output)
      valid_failure_code?(output[:failure_code]) ? output[:failure_code] : 'invalid_stage_output'
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
