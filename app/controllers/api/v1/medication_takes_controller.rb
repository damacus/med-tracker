# frozen_string_literal: true

module Api
  module V1
    class MedicationTakesController < BaseController
      def index
        render_collection(
          policy_scope(MedicationTake),
          serializer: MedicationTakeSerializer,
          includes: [{ schedule: %i[person medication] }, { person_medication: %i[person medication] }, :taken_from_location, :taken_from_medication]
        )
      end

      def create
        attributes = medication_take_params
        existing_take = idempotent_medication_take(attributes[:client_uuid])
        return render_resource(existing_take, serializer: MedicationTakeSerializer) if existing_take

        source = medication_take_source(attributes[:source_type], attributes[:source_id])
        authorize source, :take_medication?
        taken_at = parse_taken_at(attributes[:taken_at])
        return render_unprocessable('taken_at is invalid') if taken_at.blank?

        result = TakeMedicationService.new.call(
          source: source,
          amount_override: attributes[:dose_amount],
          taken_from_medication_id: attributes[:taken_from_medication_id],
          user: current_user,
          taken_at: taken_at,
          client_uuid: attributes[:client_uuid]
        )
        return render_unprocessable(take_failure_message(result.error)) unless result.success

        render_resource(result.take, serializer: MedicationTakeSerializer, status: :created)
      rescue ActiveRecord::RecordNotUnique
        take = idempotent_medication_take(attributes[:client_uuid])
        return render_resource(take, serializer: MedicationTakeSerializer) if take

        raise
      end

      private

      def medication_take_params
        params.expect(
          medication_take: %i[
            client_uuid
            source_type
            source_id
            taken_at
            dose_amount
            dose_unit
            taken_from_medication_id
          ]
        )
      end

      def idempotent_medication_take(client_uuid)
        return if client_uuid.blank?

        policy_scope(MedicationTake).find_by(client_uuid: client_uuid).tap do |take|
          authorize take, :create? if take
        end
      end

      def medication_take_source(source_type, source_id)
        case source_type
        when 'schedule'
          find_api_record(policy_scope(Schedule), source_id)
        when 'person_medication'
          find_api_record(policy_scope(PersonMedication), source_id)
        else
          raise ActiveRecord::RecordNotFound
        end
      end

      def parse_taken_at(value)
        Time.zone.parse(value.to_s)
      rescue ArgumentError, TypeError
        nil
      end

      def take_failure_message(error)
        case error
        when :out_of_stock
          'Cannot take medication: out of stock'
        when :cooldown
          'Cannot take medication: timing restrictions not met'
        when :paused
          'Cannot take medication: paused'
        when :selection_required
          'Choose a location to record this dose.'
        when :invalid_source
          'Selected location is unavailable for this medication.'
        when :invalid_amount
          'Invalid dose configured'
        else
          'Could not record medication take'
        end
      end
    end
  end
end
