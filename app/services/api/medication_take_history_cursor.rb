# frozen_string_literal: true

module Api
  class MedicationTakeHistoryCursor
    class Invalid < StandardError; end

    CIPHER = 'aes-256-gcm'
    PURPOSE = 'api.v1.medication_take_history'
    VERSION = 1

    def initialize(household:)
      @household = household
    end

    def encode(take, filter_digest:)
      encryptor.encrypt_and_sign(
        {
          'version' => VERSION,
          'household_id' => household.id,
          'taken_at' => take.taken_at.iso8601(6),
          'id' => take.id,
          'filter_digest' => filter_digest
        },
        expires_in: 1.hour,
        purpose: PURPOSE
      )
    end

    def decode(value, filter_digest:)
      payload = encryptor.decrypt_and_verify(value, purpose: PURPOSE)
      validate_payload!(payload, filter_digest:)

      [Time.iso8601(payload.fetch('taken_at')), Integer(payload.fetch('id'))]
    rescue ActiveSupport::MessageEncryptor::InvalidMessage, KeyError, ArgumentError, TypeError
      raise Invalid
    end

    private

    attr_reader :household

    def validate_payload!(payload, filter_digest:)
      valid = payload.is_a?(Hash) &&
              payload['version'] == VERSION &&
              payload['household_id'] == household.id &&
              payload['filter_digest'] == filter_digest
      raise Invalid unless valid
    end

    def encryptor
      @encryptor ||= ActiveSupport::MessageEncryptor.new(
        Rails.application.key_generator.generate_key(PURPOSE, ActiveSupport::MessageEncryptor.key_len(CIPHER)),
        cipher: CIPHER,
        serializer: JSON
      )
    end
  end
end
