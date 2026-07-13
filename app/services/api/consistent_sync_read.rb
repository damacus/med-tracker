# frozen_string_literal: true

module Api
  class ConsistentSyncRead
    def initialize(household:)
      @household = household
    end

    def call
      ActiveRecord::Base.transaction(requires_new: true) do
        household.lock!
        yield Time.current.iso8601
      end
    end

    private

    attr_reader :household
  end
end
