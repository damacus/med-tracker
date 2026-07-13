# frozen_string_literal: true

module Api
  class SyncSnapshot
    def initialize(household:, exporter:)
      @household = household
      @exporter = exporter
    end

    def payload
      Api::ConsistentSyncRead.new(household: household).call do |cursor|
        exporter.mobile_payload.merge(format: 'medtracker.portable.v2', cursor: cursor)
      end
    end

    private

    attr_reader :household, :exporter
  end
end
