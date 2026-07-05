# frozen_string_literal: true

module Api
  module V1
    class NotificationPreferenceSerializer
      def initialize(preference)
        @preference = preference
      end

      def as_json(*)
        base_attributes.merge(period_times)
      end

      private

      attr_reader :preference

      def base_attributes
        identity_attributes.merge(notification_flags).merge(updated_at: preference.updated_at.iso8601)
      end

      def identity_attributes
        {
          id: preference.id,
          portable_id: preference.portable_id,
          person_id: preference.person_id,
          person_portable_id: preference.person&.portable_id
        }
      end

      def notification_flags
        {
          enabled: preference.enabled,
          dose_due_enabled: preference.dose_due_enabled,
          missed_dose_enabled: preference.missed_dose_enabled,
          low_stock_enabled: preference.low_stock_enabled,
          private_text_enabled: preference.private_text_enabled
        }
      end

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
