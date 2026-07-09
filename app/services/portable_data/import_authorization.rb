# frozen_string_literal: true

module PortableData
  class ImportAuthorization
    def initialize(household:, membership:, payload:)
      @household = household
      @membership = membership
      @payload = payload
    end

    def call
      return if membership&.owner? || membership&.administrator?
      return if delegated_import_authorized?

      raise Pundit::NotAuthorizedError
    end

    private

    attr_reader :household, :membership, :payload

    def delegated_import_authorized?
      payload_person_ids = referenced_person_portable_ids
      manageable_ids = manageable_payload_person_ids(payload_person_ids)
      payload_person_ids.present? &&
        (payload_person_ids - manageable_ids).empty? &&
        record_writes_authorized?(manageable_ids)
    end

    def referenced_person_portable_ids
      ids = records(:people).pluck(:portable_id)
      ids.concat(records(:schedules).pluck(:person_portable_id))
      ids.concat(records(:person_medications).pluck(:person_portable_id))
      ids.concat(records(:notification_preferences).pluck(:person_portable_id))
      ids.concat(medication_take_person_portable_ids)
      ids.compact_blank.uniq
    end

    def medication_take_person_portable_ids
      records(:medication_takes).filter_map do |row|
        portable_id_for_medication_take_source(row)
      end
    end

    def portable_id_for_medication_take_source(row)
      source_portable_id = row[:source_portable_id]
      case row[:source_type]
      when 'schedule'
        person_portable_id_for_schedule(source_portable_id)
      when 'person_medication'
        person_portable_id_for_person_medication(source_portable_id)
      end
    end

    def person_portable_id_for_schedule(portable_id)
      schedule_row_for(portable_id)&.fetch(:person_portable_id, nil) ||
        Schedule.find_by(household: household, portable_id: portable_id)&.person&.portable_id
    end

    def person_portable_id_for_person_medication(portable_id)
      person_medication_row_for(portable_id)&.fetch(:person_portable_id, nil) ||
        PersonMedication.find_by(household: household, portable_id: portable_id)&.person&.portable_id
    end

    def schedule_row_for(portable_id)
      records(:schedules).find { |row| row[:portable_id] == portable_id }
    end

    def person_medication_row_for(portable_id)
      records(:person_medications).find { |row| row[:portable_id] == portable_id }
    end

    def manageable_payload_person_ids(payload_person_ids)
      Person.joins(:person_access_grants)
            .where(household: household, portable_id: payload_person_ids)
            .where(person_access_grants: manage_grant_conditions)
            .pluck(:portable_id)
    end

    def manage_grant_conditions
      {
        household: household,
        household_membership: membership,
        access_level: :manage,
        revoked_at: nil
      }
    end

    def record_writes_authorized?(manageable_person_portable_ids)
      authorized_ids = medication_portable_ids_for(manageable_person_portable_ids)
      records(:locations).empty? &&
        medication_rows_authorized?(authorized_ids) &&
        dosage_rows_authorized?(authorized_ids)
    end

    def medication_portable_ids_for(manageable_person_portable_ids)
      (
        payload_medication_portable_ids_for(manageable_person_portable_ids) +
        existing_medication_portable_ids_for(manageable_person_portable_ids)
      ).compact_blank.uniq
    end

    def payload_medication_portable_ids_for(manageable_person_portable_ids)
      schedule_ids = records(:schedules).filter_map do |row|
        row[:medication_portable_id] if manageable_person_portable_ids.include?(row[:person_portable_id])
      end
      person_medication_ids = records(:person_medications).filter_map do |row|
        row[:medication_portable_id] if manageable_person_portable_ids.include?(row[:person_portable_id])
      end

      schedule_ids + person_medication_ids
    end

    def existing_medication_portable_ids_for(manageable_person_portable_ids)
      schedule_medication_portable_ids_for(manageable_person_portable_ids) +
        person_medication_portable_ids_for(manageable_person_portable_ids)
    end

    def schedule_medication_portable_ids_for(manageable_person_portable_ids)
      Schedule.joins(:person, :medication)
              .where(household: household, people: { portable_id: manageable_person_portable_ids })
              .pluck('medications.portable_id')
    end

    def person_medication_portable_ids_for(manageable_person_portable_ids)
      PersonMedication.joins(:person, :medication)
                      .where(household: household, people: { portable_id: manageable_person_portable_ids })
                      .pluck('medications.portable_id')
    end

    def medication_rows_authorized?(authorized_medication_portable_ids)
      records(:medications).all? do |row|
        authorized_medication_portable_ids.include?(row[:portable_id])
      end
    end

    def dosage_rows_authorized?(authorized_medication_portable_ids)
      records(:dosage_options).all? do |row|
        authorized_medication_portable_ids.include?(row[:medication_portable_id])
      end
    end

    def records(name)
      Array(payload.dig(:records, name)).map(&:with_indifferent_access)
    end
  end
end
