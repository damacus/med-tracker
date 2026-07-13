# frozen_string_literal: true

module Api
  module V1
    class PersonMedicationsController < BaseController
      def index
        authorize PersonMedication
        render_collection(policy_scope(PersonMedication), serializer: PersonMedicationSerializer, includes: %i[person medication])
      end

      def show
        person_medication = find_api_record(policy_scope(PersonMedication).includes(:person, :medication),
                                            params.expect(:id))
        authorize person_medication

        render_resource(person_medication, serializer: PersonMedicationSerializer)
      end

      def create
        attributes = person_medication_params
        person = policy_scope(Person).find(attributes.delete(:person_id))
        authorize person, :show?
        assert_medication_access!(attributes[:medication_id])
        person_medication = person.person_medications.build(attributes)
        authorize person_medication

        return render_validation_errors(person_medication) unless person_medication.save

        render_resource(person_medication.reload, serializer: PersonMedicationSerializer, status: :created)
      end

      def update
        person_medication = find_api_record(policy_scope(PersonMedication).includes(:person, :medication),
                                            params.expect(:id))
        authorize person_medication
        return unless fresh_api_record?(person_medication)

        attributes = person_medication_update_params
        assert_medication_access!(attributes[:medication_id]) if attributes[:medication_id].present?

        return render_validation_errors(person_medication) unless person_medication.update(attributes)

        render_resource(person_medication.reload, serializer: PersonMedicationSerializer)
      end

      def pause
        update_pause_state(:pause!)
      end

      def resume
        update_pause_state(:resume!)
      end

      def reorder
        person_medication = find_api_record(policy_scope(PersonMedication).includes(:person, :medication),
                                            params.expect(:id))
        authorize person_medication, :update?
        PersonMedicationReorderService.new.call(
          person_medication: person_medication,
          direction: params.expect(:direction)
        )
        render_resource(person_medication, serializer: PersonMedicationSerializer)
      end

      private

      def update_pause_state(method_name)
        person_medication = find_api_record(policy_scope(PersonMedication).includes(:person, :medication),
                                            params.expect(:id))
        authorize person_medication, :update?
        person_medication.public_send(method_name)
        render_resource(person_medication, serializer: PersonMedicationSerializer)
      end

      def person_medication_params
        attributes = params.expect(
          person_medication: %i[
            person_id
            medication_id
            dose_amount
            dose_unit
            source_dosage_option_id
            administration_kind
            notes
            max_daily_doses
            min_hours_between_doses
            dose_cycle
          ]
        )
        if attributes[:person_id].present?
          attributes[:person_id] = api_record_id(policy_scope(Person), attributes[:person_id])
        end
        if attributes[:medication_id].present?
          attributes[:medication_id] = api_record_id(policy_scope(Medication), attributes[:medication_id])
        end
        if attributes[:source_dosage_option_id].present?
          attributes[:source_dosage_option_id] = api_record_id(
            policy_scope(MedicationDosageOption),
            attributes[:source_dosage_option_id]
          )
        end
        attributes
      end

      def person_medication_update_params
        person_medication_params.except(:person_id)
      end

      def assert_medication_access!(medication_id)
        policy_scope(Medication).find(medication_id)
      end
    end
  end
end
