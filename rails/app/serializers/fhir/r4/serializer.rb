# frozen_string_literal: true

module Fhir
  module R4
    class Serializer
      class << self
        def bundle(records, type:, total: records.size, links: [])
          {
            resourceType: 'Bundle',
            type: 'searchset',
            total: total,
            link: links,
            entry: records.map { |record| { resource: public_send(type, record), search: { mode: 'match' } } }
          }
        end

        def patient(person)
          {
            resourceType: 'Patient',
            id: person.portable_id,
            meta: meta(person),
            name: [{ text: person.name }],
            birthDate: person.date_of_birth&.iso8601
          }.compact
        end

        def medication(medication)
          {
            resourceType: 'Medication',
            id: medication.portable_id,
            meta: meta(medication),
            code: codeable_concept(medication),
            form: { text: medication.category }
          }.compact
        end

        def medication_request(schedule)
          {
            resourceType: 'MedicationRequest',
            id: schedule.portable_id,
            meta: meta(schedule),
            status: schedule.active? ? 'active' : 'stopped',
            intent: 'order',
            subject: reference('Patient', schedule.person),
            medicationReference: reference('Medication', schedule.medication),
            dosageInstruction: [dosage(schedule)]
          }
        end

        def medication_statement(person_medication)
          {
            resourceType: 'MedicationStatement',
            id: person_medication.portable_id,
            meta: meta(person_medication),
            status: person_medication.active? ? 'active' : 'stopped',
            subject: reference('Patient', person_medication.person),
            medicationReference: reference('Medication', person_medication.medication),
            dosage: [dosage(person_medication)]
          }
        end

        def medication_administration(take)
          source = take.schedule || take.person_medication
          medication = take.taken_from_medication || source&.medication
          {
            resourceType: 'MedicationAdministration',
            id: take.portable_id,
            meta: meta(take),
            status: 'completed',
            subject: reference('Patient', source&.person),
            medicationReference: reference('Medication', medication),
            effectiveDateTime: take.taken_at&.iso8601
          }.compact
        end

        private

        def reference(resource_type, record)
          return if record.blank?

          { reference: "#{resource_type}/#{record.portable_id}" }
        end

        def meta(record)
          return unless record.respond_to?(:updated_at)
          return if record.updated_at.blank?

          { lastUpdated: record.updated_at.iso8601 }
        end

        def codeable_concept(medication)
          concept = { text: medication.display_name }
          coding = medication_coding(medication)
          concept[:coding] = [coding] if coding.present?
          concept
        end

        def medication_coding(medication)
          return if medication.dmd_code.blank?

          {
            system: medication.dmd_system,
            code: medication.dmd_code,
            display: medication.display_name,
            userSelected: true
          }.compact
        end

        def dosage(record)
          {
            text: [record.dose_amount, record.dose_unit, record.try(:frequency)].compact_blank.join(' '),
            doseAndRate: [
              {
                doseQuantity: {
                  value: record.dose_amount&.to_f,
                  unit: record.dose_unit
                }.compact
              }
            ]
          }
        end
      end
    end
  end
end
