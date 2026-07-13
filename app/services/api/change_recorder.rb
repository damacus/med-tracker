# frozen_string_literal: true

module Api
  class ChangeRecorder
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
        metadata: metadata_for(record)
      }
    end

    def metadata_for(record)
      {
        record_type: record.class.name,
        record_id: record.id,
        portable_id: portable_id(record)
      }.compact
    end

    def portable_id(record)
      record.portable_id if record.respond_to?(:portable_id)
    end
  end
end
