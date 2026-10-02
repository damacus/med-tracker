# frozen_string_literal: true

module PortableData
  class Exporter
    FORMAT = 'medtracker.portable.v1'
    V2_FORMAT = 'medtracker.portable.v2'

    class Error < StandardError; end

    def initialize(household:, membership:, passphrase:, person_ids: nil, **options)
      @household = household
      @membership = membership
      @passphrase = passphrase
      @person_ids = Array(person_ids).compact_blank
      @request = options[:request]
      @people_scope = options[:people_scope]
    end

    def call(version: 1, format: FORMAT)
      export_payload = payload(format: version == 2 ? V2_FORMAT : format)
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

    def payload(format: FORMAT)
      export_payload(format: format)
    end

    def mobile_payload(format: FORMAT)
      export_payload(include_health_events: true, format: format)
    end

    def v2_payload(include_health_events: false)
      export_payload(include_health_events: include_health_events, format: V2_FORMAT)
    end

    def mobile_v2_payload = v2_payload(include_health_events: true)

    def household_payload
      export_payload(include_health_events: true, scope: 'household', format: V2_FORMAT)
    end

    private

    attr_reader :household, :membership, :passphrase, :person_ids, :request

    def export_payload(include_health_events: false, scope: 'single_person', format: FORMAT)
      raise Error, 'Unsupported portable data format' unless [FORMAT, V2_FORMAT].include?(format)

      {
        format: format,
        scope: scope,
        exported_at: Time.current.iso8601,
        source_instance_id: source_instance_id,
        records: records_payload(
          include_health_events: include_health_events,
          household_wide: scope == 'household',
          include_outcomes: format == V2_FORMAT
        )
      }
    end

    def records_payload(include_health_events:, household_wide:, include_outcomes: false)
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
      records[:dose_occurrences] = dose_occurrences if include_outcomes
      records[:medication_pause_periods] = medication_pause_periods if include_outcomes
      ExportRecordSerializer.new(records).as_json
    end

    def source_instance_id
      "hosted:#{Rails.env}"
    end

    def people
      @people ||= begin
        scope = household.people.where(id: @people_scope || manageable_person_ids).order(:id)
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

    def medication_pause_periods
      scheduled = MedicationPausePeriod.where(household: household, schedule_id: schedules.select(:id))
      assigned = MedicationPausePeriod.where(household: household, person_medication_id: person_medications.select(:id))
      scheduled.or(assigned).includes(:schedule, :person_medication,
                                      recorded_by_membership: :person, resumed_by_membership: :person).order(:id)
    end

    def dose_occurrences
      scope = MedicationDoseOccurrence.where(household: household)
      scope.where(schedule_id: schedules.select(:id))
           .or(scope.where(person_medication_id: person_medications.select(:id)))
           .includes(:schedule, :person_medication, :medication_take).order(:id)
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
