# frozen_string_literal: true

module Api
  class ChangeRecorder
    PERSON_ASSOCIATED_RECORD_TYPES = [HealthEvent, NotificationPreference, PersonMedication, Schedule].freeze

    def initialize(household:, membership:, account: nil, request_id: nil)
      @household = household
      @account = account
      @membership = membership
      @request_id = request_id
    end

    def record(record, action:)
      return unless recordable?(record)

      household.lock!
      return ApiTombstone.create!(tombstone_attributes(record)) if action == 'delete'

      ApiChangeEvent.create!(change_attributes(record, action))
    end

    def tombstone_visibility_metadata_for(record)
      return {} unless record.is_a?(Medication)

      person_ids = record.schedules.pluck(:person_id) + record.person_medications.pluck(:person_id)
      person_ids.uniq!
      if person_ids.any?
        { sync_person_portable_ids: Person.where(id: person_ids).pluck(:portable_id) }
      elsif record.created_by_membership_id.present?
        { sync_creator_membership_id: record.created_by_membership_id.to_s }
      else
        {}
      end
    end

    private

    attr_reader :household, :account, :membership, :request_id

    def recordable?(record)
      return false unless household && record
      return false unless record.respond_to?(:household_id) && record.respond_to?(:portable_id)

      record.household_id == household.id && record.portable_id.present?
    end

    def change_attributes(record, action)
      {
        household: household,
        account: account,
        household_membership: membership,
        request_id: request_id,
        record_type: record.class.name,
        record_id: record.id,
        record_portable_id: portable_id(record),
        action: action,
        occurred_at: Time.current,
        metadata: metadata_for(record)
      }
    end

    def tombstone_attributes(record)
      {
        household: household,
        account: account,
        household_membership: membership,
        record_type: record.class.name,
        record_portable_id: record.portable_id,
        action: 'delete',
        deleted_at: Time.current,
        metadata: metadata_for(record).merge(record.sync_tombstone_visibility_metadata ||
                                             tombstone_visibility_metadata_for(record))
      }
    end

    def metadata_for(record)
      {
        record_type: record.class.name,
        record_id: record.id,
        portable_id: portable_id(record)
      }.compact.merge(person_metadata(record))
    end

    def person_metadata(record)
      person = person_for(record)

      return {} unless person

      { person_portable_id: person.portable_id }
    end

    def person_for(record)
      person_record_person(record) || source_person(record)
    end

    def person_record_person(record)
      return record if record.is_a?(Person)
      return unless PERSON_ASSOCIATED_RECORD_TYPES.any? { |record_type| record.is_a?(record_type) }

      record.person
    end

    def source_person(record)
      case record
      when MedicationDoseOccurrence
        record.source&.person
      when MedicationPausePeriod, MedicationTake
        record.schedule&.person || record.person_medication&.person
      end
    end

    def portable_id(record)
      record.portable_id if record.respond_to?(:portable_id)
    end
  end
end
