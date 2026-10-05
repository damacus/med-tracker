# frozen_string_literal: true

require 'json'

module Audit
  module Verification
    class Command
      SCOPES = %w[database worm combined].freeze

      def initialize(environment: ENV, output: $stdout, error_output: $stderr, database_verifier: nil,
                     worm_verifier: nil)
        @environment = environment
        @output = output
        @error_output = error_output
        @database_verifier = database_verifier
        @worm_verifier = worm_verifier
      end

      def call
        result = verification_result
        render(result)
        result.exit_code
      rescue ConfigurationError, Audit::EntryFilter::Invalid => e
        error_output.puts("audit verification could not run: #{e.message}")
        2
      rescue StandardError => e
        error_output.puts("audit verification could not run: #{e.class.name}")
        2
      end

      private

      attr_reader :environment, :output, :error_output, :database_verifier, :worm_verifier

      def verification_result
        raise ConfigurationError, "unsupported scope: #{scope}" unless SCOPES.include?(scope)

        validate_filter_compatibility!

        case scope
        when 'database' then resolved_database_verifier.call
        when 'worm' then resolved_worm_verifier.call
        else combine(resolved_database_verifier.call, resolved_worm_verifier.call)
        end
      end

      def resolved_database_verifier
        database_verifier || DatabaseVerifier.new(entries: entry_filter.call,
                                                  household_id: entry_filter.household_id)
      end

      def resolved_worm_verifier
        worm_verifier || raise(ConfigurationError, 'WORM verification is not configured')
      end

      def entry_filter
        @entry_filter ||= Audit::EntryFilter.new(environment)
      end

      def scope
        @scope ||= environment.fetch('SCOPE', 'database').downcase
      end

      def validate_filter_compatibility!
        return if scope == 'worm' || !entry_filter.time_filtered?

        raise ConfigurationError, 'time filters are unsupported for database verification'
      end

      def combine(database_result, worm_result)
        Result.new(
          scope: 'combined',
          checked_entries: database_result.checked_entries,
          checked_checkpoints: database_result.checked_checkpoints,
          checked_objects: worm_result.checked_objects,
          issues: database_result.issues + worm_result.issues
        )
      end

      def render(result)
        environment.fetch('FORMAT', 'human') == 'json' ? render_json(result) : render_human(result)
      end

      def render_json(result)
        output.puts(JSON.generate(result.to_h))
      end

      def render_human(result)
        output.puts("Audit verification: #{result.valid? ? 'VALID' : 'INVALID'}")
        output.puts(result_summary(result))
        result.issues.each { |issue| output.puts("#{issue.code}: #{issue.message}") }
      end

      def result_summary(result)
        "Entries: #{result.checked_entries}; checkpoints: #{result.checked_checkpoints}; " \
          "objects: #{result.checked_objects}"
      end
    end
  end
end
