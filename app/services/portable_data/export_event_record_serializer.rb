# frozen_string_literal: true

module PortableData
  class ExportEventRecordSerializer
    def medication_take_payload(take)
      medication_take_identity(take).merge(medication_take_source(take))
                                    .merge(medication_take_event(take))
                                    .merge(medication_take_inventory(take))
    end

    def notification_preference_payload(preference)
      notification_preference_identity(preference).merge(notification_preference_flags(preference))
                                                  .merge(notification_preference_times(preference))
    end

    def health_event_payload(event)
      sync_identity(event).merge(
        person_portable_id: event.person.portable_id,
        event_kind: event.event_kind,
        severity: event.severity,
        title: event.title,
        notes: event.notes,
        started_on: event.started_on&.iso8601,
        ended_on: event.ended_on&.iso8601,
        medication_portable_ids: event.medications.map(&:portable_id)
      )
    end

    private

    def medication_take_identity(take)
      sync_identity(take).merge(client_uuid: take.client_uuid)
    end

    def medication_take_source(take)
      source = take.schedule || take.person_medication

      {
        source_type: take.schedule_id ? 'schedule' : 'person_medication',
        source_portable_id: source.portable_id
      }
    end

    def medication_take_event(take)
      {
        taken_at: take.taken_at&.iso8601(6),
        dose_amount: take.dose_amount,
        dose_unit: take.dose_unit
      }
    end

    def medication_take_inventory(take)
      {
        taken_from_medication_portable_id: take.taken_from_medication&.portable_id,
        taken_from_location_portable_id: take.taken_from_location&.portable_id
      }
    end

    def notification_preference_identity(preference)
      sync_identity(preference).merge(person_portable_id: preference.person.portable_id)
    end

    def notification_preference_flags(preference)
      {
        enabled: preference.enabled,
        dose_due_enabled: preference.dose_due_enabled,
        missed_dose_enabled: preference.missed_dose_enabled,
        low_stock_enabled: preference.low_stock_enabled,
        private_text_enabled: preference.private_text_enabled
      }
    end

    def notification_preference_times(preference)
      {
        morning_time: formatted_time(preference.morning_time),
        afternoon_time: formatted_time(preference.afternoon_time),
        evening_time: formatted_time(preference.evening_time),
        night_time: formatted_time(preference.night_time)
      }
    end

    def formatted_time(value)
      value&.strftime('%H:%M:%S')
    end

    def sync_identity(record)
      {
        portable_id: record.portable_id,
        updated_at: record.updated_at.iso8601,
        etag: Api::RecordEtag.for(record)
      }
    end
  end
end
