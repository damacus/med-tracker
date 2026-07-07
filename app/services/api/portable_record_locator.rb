# frozen_string_literal: true

module Api
  class PortableRecordLocator
    UUID_PATTERN = /\A[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/i

    def initialize(household:)
      @household = household
    end

    def find(scope, identifier)
      return scope.find(identifier) unless portable_identifier?(identifier)

      scope.where(household: household).find_by!(portable_id: identifier)
    end

    private

    attr_reader :household

    def portable_identifier?(identifier)
      identifier.to_s.match?(UUID_PATTERN)
    end
  end
end
