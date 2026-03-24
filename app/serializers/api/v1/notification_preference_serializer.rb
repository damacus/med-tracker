# frozen_string_literal: true

module Api
  module V1
    class NotificationPreferenceSerializer
      def initialize(preference)
        @preference = preference
      end

      def as_json(*)
        {
          id: preference.id,
          person_id: preference.person_id,
          enabled: preference.enabled,
          updated_at: preference.updated_at.iso8601
        }.merge(period_times)
      end

      private

      attr_reader :preference

      def period_times
        {
          morning_time: formatted_time(preference.morning_time),
          afternoon_time: formatted_time(preference.afternoon_time),
          evening_time: formatted_time(preference.evening_time),
          night_time: formatted_time(preference.night_time)
        }
      end

      def formatted_time(value)
        value&.strftime('%H:%M:%S')
      end
    end
  end
end
