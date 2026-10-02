# frozen_string_literal: true

module Api
  class SyncSnapshot
    def initialize(household:, exporter:)
      @household = household
      @exporter = exporter
    end

    def payload
      Api::ConsistentSyncRead.new(household: household).call do |cursor|
        exporter.mobile_payload(format: PortableData::Exporter::V2_FORMAT).merge(cursor: cursor)
      end
    end

    private

    attr_reader :household, :exporter
  end
end
