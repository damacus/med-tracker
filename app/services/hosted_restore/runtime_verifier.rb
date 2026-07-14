# frozen_string_literal: true

module HostedRestore
  class VerificationError < StandardError; end

  class RuntimeVerifier
    TABLES = %w[people security_audit_events active_storage_attachments].freeze
    MODELS = {
      clinical: Person,
      audit: SecurityAuditEvent,
      attachments: ActiveStorage::Attachment
    }.freeze

    def initialize(household_ids:, runtime_image: ENV.fetch('RUNTIME_APP_IMAGE'),
                   connection: ActiveRecord::Base.connection, storage_verifier: Storage::RestoreVerifier,
                   models: MODELS)
      @household_ids = household_ids.map { |value| canonical_integer(value) }
      @runtime_image = runtime_image
      @connection = connection
      @storage_verifier = storage_verifier
      @models = models
    end

    def call
      verify_role!
      verify_households!
      verify_runtime_image!
      verify_forced_rls!
      samples = representative_samples
      verify_default_deny!
      verify_isolation!(samples)
      verify_storage!(samples)

      {
        schema_version:, database_role: 'med_tracker_app', app_image: runtime_image,
        forced_rls: true, default_deny: true,
        isolation: { clinical: true, audit: true, attachments: true },
        storage: { samples_verified: samples.size }
      }
    end

    private

    attr_reader :household_ids, :runtime_image, :connection, :storage_verifier, :models

    def verify_role!
      role = connection.select_value('SELECT current_user')
      raise VerificationError, 'runtime_role_required' unless role == 'med_tracker_app'
    end

    def verify_households!
      return if household_ids.size == 2 && household_ids.all?(&:positive?) && household_ids.uniq.size == 2

      raise VerificationError, 'distinct_household_samples_required'
    end

    def verify_runtime_image!
      valid = !runtime_image.end_with?(':latest') &&
              runtime_image.match?(
                %r{\A[a-zA-Z0-9][a-zA-Z0-9._/-]*(?::[a-zA-Z0-9][a-zA-Z0-9._-]*|@sha256:[a-f0-9]{64})\z}
              )
      raise VerificationError, 'runtime_app_image_invalid' unless valid
    end

    def verify_forced_rls!
      configured = TABLES.all? do |table|
        connection.select_one(<<~SQL.squish) == { 'relrowsecurity' => true, 'relforcerowsecurity' => true }
          SELECT relrowsecurity, relforcerowsecurity
          FROM pg_class
          WHERE oid = #{connection.quote(table)}::regclass
        SQL
      end
      raise VerificationError, 'forced_rls_required' unless configured
    end

    def representative_samples
      household_ids.to_h do |household_id|
        sample = with_household(household_id) do
          models.transform_values { |model| model.where(household_id:).pick(:id) }
        end
        raise VerificationError, 'representative_sample_missing' if sample.value?(nil)

        [household_id, sample]
      end
    end

    def verify_default_deny!
      denied = without_household do
        models.values.all? { |model| model.where(household_id: household_ids).none? }
      end
      raise VerificationError, 'rls_default_deny_failed' unless denied
    end

    def verify_isolation!(samples)
      samples.each do |household_id, own_sample|
        other_sample = samples.fetch((household_ids - [household_id]).sole)
        isolated = with_household(household_id) do
          models.all? do |name, model|
            model.exists?(own_sample.fetch(name)) && !model.exists?(other_sample.fetch(name))
          end
        end
        raise VerificationError, 'cross_household_isolation_failed' unless isolated
      end
    end

    def verify_storage!(samples)
      samples.each do |household_id, sample|
        with_household(household_id) do
          storage_verifier.call(attachment_id: sample.fetch(:attachments))
        end
      end
    rescue Storage::RestoreVerifier::VerificationError
      raise VerificationError, 'storage_restore_verification_failed'
    end

    def with_household(household_id, &)
      with_setting(household_id.to_s, &)
    end

    def without_household(&)
      with_setting('', &)
    end

    def with_setting(value)
      connection.transaction(requires_new: true) do
        apply_household_setting(value)
        yield
      ensure
        apply_household_setting('')
      end
    end

    def apply_household_setting(value)
      connection.execute(
        ActiveRecord::Base.sanitize_sql_array(
          ['SELECT set_config(?, ?, true)', TenantContext::SETTING_NAMES.fetch(:household), value]
        )
      )
    end

    def schema_version
      connection.select_value('SELECT max(version) FROM schema_migrations').to_s
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
