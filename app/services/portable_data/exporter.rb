# frozen_string_literal: true

module PortableData
  class Exporter
    FORMAT = 'medtracker.portable.v1'

    def initialize(household:, membership:, passphrase:, person_ids: nil, request: nil)
      @household = household
      @membership = membership
      @passphrase = passphrase
      @person_ids = Array(person_ids).compact_blank
      @request = request
    end

    def call
      export_payload = payload
      envelope = Encryptor.encrypt(export_payload, passphrase: passphrase)
      record_audit_event(export_payload, export_mode: 'encrypted_migration_bundle')
      envelope
    end

    def export_unencrypted(export_mode:, event_type: 'portable_data.exported', export_payload: payload)
      result = yield(export_payload)
      record_audit_event(export_payload, event_type: event_type, encrypted: false, export_mode: export_mode)
      result
    end

    def mobile_snapshot
      export_unencrypted(
        export_mode: 'mobile_snapshot',
        event_type: 'portable_data.mobile_snapshot_read',
        export_payload: mobile_payload
      ) do |export_payload|
        export_payload
      end
    end

    def payload
      export_payload
    end

    def mobile_payload
      export_payload(include_health_events: true)
    end

    def household_payload
      export_payload(include_health_events: true, scope: 'household')
    end

    private

    attr_reader :household, :membership, :passphrase, :person_ids, :request

    def export_payload(include_health_events: false, scope: 'single_person')
      {
        format: FORMAT,
        scope: scope,
        exported_at: Time.current.iso8601,
        source_instance_id: source_instance_id,
        records: records_payload(
          include_health_events: include_health_events,
          household_wide: scope == 'household'
        )
      }
    end

    def records_payload(include_health_events:, household_wide:)
      records = {
        people: people,
        locations: locations(include_health_events:, household_wide:),
        medications: medications(include_health_events:, household_wide:),
        dosage_options: dosage_options(include_health_events:, household_wide:),
        schedules: schedules,
        person_medications: person_medications,
        medication_takes: medication_takes,
        notification_preferences: notification_preferences
      }
      records[:health_events] = health_events if include_health_events
      ExportRecordSerializer.new(records).as_json
    end

    def source_instance_id
      "hosted:#{Rails.env}"
    end

    def people
      @people ||= begin
        scope = household.people.where(id: manageable_person_ids).order(:id)
        person_ids.present? ? scope.where(id: person_ids) : scope
      end
    end

    def manageable_person_ids
      return household.people.select(:id) if household_manager?

      PersonAccessGrant.active
                       .where(household: household, household_membership: membership, access_level: :manage)
                       .select(:person_id)
    end

    def household_manager?
      membership&.owner? || membership&.administrator?
    end

    def person_id_values
      @person_id_values ||= people.pluck(:id)
    end

    def location_id_values(include_health_events:)
      @location_id_values ||= {}
      @location_id_values[include_health_events] ||= begin
        ids = LocationMembership.where(household: household, person_id: person_id_values).pluck(:location_id)
        ids.concat(medications(include_health_events:, household_wide: false).pluck(:location_id))
        ids.concat(medication_takes.pluck(:taken_from_location_id))
        ids.compact.uniq
      end
    end

    def medication_id_values(include_health_events:)
      @medication_id_values ||= {}
      @medication_id_values[include_health_events] ||= begin
        ids = schedules.pluck(:medication_id)
        ids.concat(person_medications.pluck(:medication_id))
        if include_health_events
          ids.concat(health_events.joins(:health_event_medications).pluck('health_event_medications.medication_id'))
        end
        ids.compact.uniq
      end
    end

    def locations(include_health_events:, household_wide:)
      @locations ||= {}
      @locations[[include_health_events, household_wide]] ||= begin
        scope = household.locations
        scope = scope.where(id: location_id_values(include_health_events:)) unless household_wide
        scope.order(:id)
      end
    end

    def medications(include_health_events:, household_wide:)
      @medications ||= {}
      @medications[[include_health_events, household_wide]] ||= begin
        scope = household.medications
        scope = scope.where(id: medication_id_values(include_health_events:)) unless household_wide
        scope.includes(:location).order(:id)
      end
    end

    def dosage_options(include_health_events:, household_wide:)
      @dosage_options ||= {}
      @dosage_options[[include_health_events, household_wide]] ||= begin
        scope = MedicationDosageOption.where(household: household)
        scope = scope.where(medication_id: medication_id_values(include_health_events:)) unless household_wide
        scope.includes(:medication).order(:id)
      end
    end

    def schedules
      @schedules ||= Schedule.where(household: household, person_id: person_id_values)
                             .includes(:person, :medication, :source_dosage_option)
                             .order(:id)
    end

    def person_medications
      @person_medications ||= PersonMedication.where(household: household, person_id: person_id_values)
                                              .includes(:person, :medication, :source_dosage_option)
                                              .order(:id)
    end

    def medication_takes
      @medication_takes ||= begin
        scheduled = MedicationTake.where(household: household, schedule_id: schedules.select(:id))
        unscheduled = MedicationTake.where(household: household, person_medication_id: person_medications.select(:id))
        scheduled.or(unscheduled)
                 .includes(:schedule, :person_medication, :taken_from_medication, :taken_from_location)
                 .order(:id)
      end
    end

    def notification_preferences
      @notification_preferences ||= NotificationPreference.where(household: household, person_id: person_id_values)
                                                          .includes(:person)
                                                          .order(:id)
    end

    def health_events
      @health_events ||= HealthEvent.where(household: household, person_id: person_id_values)
                                    .includes(:person, :medications)
                                    .order(:id)
    end

    def record_audit_event(payload, export_mode:, event_type: 'portable_data.exported', encrypted: true)
      Audit::Event.record!(
        household: household,
        actor_account: membership.account,
        actor_membership: membership,
        event_type: event_type,
        request: request,
        metadata: {
          record_counts: record_counts(payload),
          encrypted: encrypted,
          export_mode: export_mode
        }
      )
    end

    def record_counts(payload)
      payload.fetch(:records).transform_values(&:size)
    end
  end
end
