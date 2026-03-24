# frozen_string_literal: true

module Api
  module V1
    class LocationSerializer
      def initialize(location)
        @location = location
      end

      def as_json(*)
        {
          id: location.id,
          name: location.name,
          description: location.description,
          updated_at: location.updated_at.iso8601
        }
      end

      private

      attr_reader :location
    end
  end
end
