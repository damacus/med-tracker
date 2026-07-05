# frozen_string_literal: true

module PortableData
  class ExportRecordSerializer
    COLLECTIONS = {
      people: :person_payload,
      locations: :location_payload,
      medications: :medication_payload,
      dosage_options: :dosage_option_payload,
      schedules: :schedule_payload,
      person_medications: :person_medication_payload,
      medication_takes: :event_medication_take_payload,
      notification_preferences: :event_notification_preference_payload
    }.freeze

    def initialize(records)
      @records = records
    end

    def as_json
      COLLECTIONS.to_h do |record_type, serializer_method|
        [record_type, records.fetch(record_type).map { |record| send(serializer_method, record) }]
      end
    end

    private

    attr_reader :records

    def person_payload(person)
      person_identity(person).merge(person_profile(person)).merge(person_relationships(person))
    end

    def person_identity(person)
      {
        portable_id: person.portable_id,
        updated_at: person.updated_at.iso8601
      }
    end

    def person_profile(person)
      {
        name: person.name,
        email: person.email,
        date_of_birth: person.date_of_birth&.iso8601,
        person_type: person.person_type,
        has_capacity: person.has_capacity
      }
    end

    def person_relationships(person)
      {
        location_portable_ids: person.locations.map(&:portable_id),
        notification_preference_portable_id: person.notification_preference&.portable_id
      }
    end

    def location_payload(location)
      {
        portable_id: location.portable_id,
        name: location.name,
        description: location.description,
        updated_at: location.updated_at.iso8601
      }
    end

    def medication_payload(medication)
      medication_identity(medication).merge(medication_dose(medication))
                                     .merge(medication_inventory(medication))
                                     .merge(medication_catalog(medication))
    end

    def medication_identity(medication)
      {
        portable_id: medication.portable_id,
        location_portable_id: medication.location&.portable_id,
        name: medication.name,
        friendly_name: medication.friendly_name,
        category: medication.category,
        description: medication.description,
        updated_at: medication.updated_at.iso8601
      }
    end

    def medication_dose(medication)
      {
        dose_amount: medication.dose_amount,
        dose_unit: medication.dose_unit,
        default_schedule_type: medication.default_schedule_type
      }
    end

    def medication_inventory(medication)
      {
        current_supply: medication.current_supply,
        reorder_threshold: medication.reorder_threshold
      }
    end

    def medication_catalog(medication)
      {
        barcode: medication.barcode,
        dmd_code: medication.dmd_code,
        dmd_system: medication.dmd_system,
        dmd_concept_class: medication.dmd_concept_class
      }
    end

    def dosage_option_payload(dosage)
      dosage_identity(dosage).merge(dosage_defaults(dosage)).merge(dosage_inventory(dosage))
    end

    def dosage_identity(dosage)
      {
        portable_id: dosage.portable_id,
        medication_portable_id: dosage.medication.portable_id,
        amount: dosage.amount,
        unit: dosage.unit,
        frequency: dosage.frequency,
        description: dosage.description,
        updated_at: dosage.updated_at.iso8601
      }
    end

    def dosage_defaults(dosage)
      {
        default_for_adults: dosage.default_for_adults,
        default_for_children: dosage.default_for_children,
        default_max_daily_doses: dosage.default_max_daily_doses,
        default_min_hours_between_doses: dosage.default_min_hours_between_doses,
        default_dose_cycle: dosage.default_dose_cycle
      }
    end

    def dosage_inventory(dosage)
      {
        current_supply: dosage.current_supply,
        reorder_threshold: dosage.reorder_threshold
      }
    end

    def schedule_payload(schedule)
      schedule_identity(schedule).merge(schedule_subjects(schedule))
                                 .merge(schedule_dose(schedule))
                                 .merge(schedule_timing(schedule))
    end

    def schedule_identity(schedule)
      {
        portable_id: schedule.portable_id,
        source_dosage_option_portable_id: schedule.source_dosage_option&.portable_id,
        updated_at: schedule.updated_at.iso8601
      }
    end

    def schedule_subjects(schedule)
      {
        person_portable_id: schedule.person.portable_id,
        medication_portable_id: schedule.medication.portable_id
      }
    end

    def schedule_dose(schedule)
      {
        dose_amount: schedule.dose_amount,
        dose_unit: schedule.dose_unit,
        frequency: schedule.frequency,
        dose_cycle: schedule.dose_cycle,
        max_daily_doses: schedule.max_daily_doses,
        min_hours_between_doses: schedule.min_hours_between_doses
      }
    end

    def schedule_timing(schedule)
      {
        schedule_type: schedule.schedule_type,
        schedule_config: schedule.schedule_config,
        start_date: schedule.start_date&.iso8601,
        end_date: schedule.end_date&.iso8601,
        active: schedule.active,
        notes: schedule.notes
      }
    end

    def person_medication_payload(person_medication)
      person_medication_identity(person_medication).merge(person_medication_subjects(person_medication))
                                                   .merge(person_medication_dose(person_medication))
                                                   .merge(person_medication_state(person_medication))
    end

    def person_medication_identity(person_medication)
      {
        portable_id: person_medication.portable_id,
        source_dosage_option_portable_id: person_medication.source_dosage_option&.portable_id,
        updated_at: person_medication.updated_at.iso8601
      }
    end

    def person_medication_subjects(person_medication)
      {
        person_portable_id: person_medication.person.portable_id,
        medication_portable_id: person_medication.medication.portable_id
      }
    end

    def person_medication_dose(person_medication)
      {
        dose_amount: person_medication.dose_amount,
        dose_unit: person_medication.dose_unit,
        dose_cycle: person_medication.dose_cycle,
        max_daily_doses: person_medication.max_daily_doses,
        min_hours_between_doses: person_medication.min_hours_between_doses
      }
    end

    def person_medication_state(person_medication)
      {
        administration_kind: person_medication.administration_kind,
        active: person_medication.active,
        notes: person_medication.notes,
        position: person_medication.position
      }
    end

    def event_medication_take_payload(take)
      event_record_serializer.medication_take_payload(take)
    end

    def event_notification_preference_payload(preference)
      event_record_serializer.notification_preference_payload(preference)
    end

    def event_record_serializer
      @event_record_serializer ||= ExportEventRecordSerializer.new
    end
  end
end
