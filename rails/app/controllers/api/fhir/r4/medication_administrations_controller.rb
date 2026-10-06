# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationAdministrationsController < BaseController
        def index
          scope = policy_scope(MedicationTake)
                  .includes({ schedule: %i[person medication] }, { person_medication: %i[person medication] },
                            :taken_from_medication)
                  .order(:id)
          render_fhir_collection(scope, :medication_administration, search: search)
        end

        def show
          take = find_api_record(
            policy_scope(MedicationTake).includes({ schedule: %i[person medication] },
                                                  { person_medication: %i[person medication] },
                                                  :taken_from_medication),
            params.expect(:id)
          )
          render_fhir_resource(take, :medication_administration)
        end

        private

        def search
          {
            _id: search_by_portable_id,
            patient: patient_search,
            subject: patient_search,
            medication: medication_search,
            status: status_search,
            date: lambda { |scope, value|
              scope.where(taken_at: iso8601_date(value, field: 'date').all_day)
            }
          }
        end

        def patient_search
          lambda do |scope, value|
            person = Person.find_by!(portable_id: portable_reference_id(value), household: current_household)
            scope.left_outer_joins(:schedule, :person_medication)
                 .where(schedules: { person_id: person.id })
                 .or(scope.left_outer_joins(:schedule, :person_medication)
                          .where(person_medications: { person_id: person.id }))
          end
        end

        def medication_search
          lambda do |scope, value|
            medication = Medication.find_by!(portable_id: portable_reference_id(value), household: current_household)
            scope.left_outer_joins(:schedule, :person_medication)
                 .where(schedules: { medication_id: medication.id })
                 .or(scope.left_outer_joins(:schedule, :person_medication)
                          .where(person_medications: { medication_id: medication.id }))
          end
        end

        def status_search
          lambda do |scope, value|
            raise InvalidFilterValue, 'status must be completed' unless value.to_s == 'completed'

            scope
          end
        end
      end
    end
  end
end
