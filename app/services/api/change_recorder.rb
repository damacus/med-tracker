# frozen_string_literal: true

module Api
  class ChangeRecorder
    def initialize(household:, credential:, membership:, request:)
      @household = household
      @credential = credential
      @membership = membership
      @request = request
    end

    def record(record, action:)
      return unless recordable?(record)

      ApiChangeEvent.create!(change_attributes(record, action))
    end

    private

    attr_reader :household, :credential, :membership, :request

    def recordable?(record)
      household && credential&.account && membership && record&.persisted?
    end

    def change_attributes(record, action)
      {
        household: household,
        account: credential.account,
        household_membership: membership,
        request_id: request.request_id,
        record_type: record.class.name,
        record_id: record.id,
        record_portable_id: portable_id(record),
        action: action,
        occurred_at: Time.current,
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
