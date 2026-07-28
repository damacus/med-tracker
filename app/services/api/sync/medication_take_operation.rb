# frozen_string_literal: true

module Api
  module Sync
    class MedicationTakeOperation
      CLIENT_UUID_INDEX = 'index_medication_takes_on_client_uuid'
      STOCK_ERRORS = %i[out_of_stock].freeze
      TIMING_ERRORS = %i[cooldown paused overlapping_prescription_restriction].freeze

      Result = Data.define(:take, :replayed)

      class Error < StandardError
        attr_reader :code, :status

        def initialize(code:, message:, status: :unprocessable_content)
          @code = code
          @status = status
          super(message)
        end
      end

      class RetryBatch < StandardError
        attr_reader :reason

        def initialize(reason)
          @reason = reason
          super()
        end

        def client_uuid_constraint?
          reason == :client_uuid_constraint
        end
      end

      def call(attributes:, user:, existing_take: nil, route: nil, &source_resolver)
        attributes = attributes.to_h.symbolize_keys
        client_uuid = attributes[:client_uuid].to_s
        validate_client_uuid!(client_uuid)
        return Result.new(take: existing_take, replayed: true) if existing_take

        context = { client_uuid: client_uuid, user: user, route: route }
        record_new_take(attributes:, context:, &source_resolver)
      rescue ActiveRecord::RecordNotUnique => e
        raise unless e.message.include?(CLIENT_UUID_INDEX)

        raise RetryBatch, :client_uuid_constraint
      end

      private

      def record_new_take(attributes:, context:)
        taken_at = parse_taken_at(attributes[:taken_at])
        validate_source_reference!(attributes)
        source = yield(attributes[:source_type], attributes[:source_id])
        result = record_dose(source:, attributes:, taken_at:, context:)
        build_result(result, source:, client_uuid: context.fetch(:client_uuid))
      end

      def build_result(result, source:, client_uuid:)
        raise RetryBatch, persistence_failure_reason(source:, client_uuid:) if result.error == :create_failed
        raise domain_error(result.error) unless result.success

        Result.new(take: result.take, replayed: false)
      end

      def persistence_failure_reason(source:, client_uuid:)
        collision = MedicationTake.exists?(household_id: source.household_id, client_uuid: client_uuid)
        return :client_uuid_constraint if collision

        :persistence_failure
      end

      def validate_client_uuid!(client_uuid)
        raise invalid_error('client_uuid is required') if client_uuid.blank?
      end

      def parse_taken_at(value)
        Time.zone.parse(value.to_s).tap do |taken_at|
          raise invalid_error('taken_at is invalid') if taken_at.blank?
        end
      rescue ArgumentError, TypeError
        raise invalid_error('taken_at is invalid')
      end

      def validate_source_reference!(attributes)
        return if attributes[:source_type].present? && attributes[:source_id].present?

        raise invalid_error('source_type and source_id are required')
      end

      def record_dose(source:, attributes:, taken_at:, context:)
        MedicationAdministration::RecordDose.new.call(
          source: source,
          amount_override: attributes[:dose_amount],
          taken_from_medication_id: attributes[:taken_from_medication_id],
          user: context.fetch(:user),
          taken_at: taken_at,
          client_uuid: context.fetch(:client_uuid),
          route: context.fetch(:route)
        )
      end

      def domain_error(error)
        return stock_error if STOCK_ERRORS.include?(error)
        return timing_error if TIMING_ERRORS.include?(error)

        invalid_error('Medication take is invalid')
      end

      def stock_error
        Error.new(
          code: 'medication_stock_unavailable',
          message: 'Medication stock is unavailable'
        )
      end

      def timing_error
        Error.new(
          code: 'medication_timing_conflict',
          message: 'Medication timing rules prevent this dose'
        )
      end

      def invalid_error(message)
        Error.new(code: 'medication_take_invalid', message: message)
      end
    end
  end
end
