# frozen_string_literal: true

module Api
  module V1
    class NotificationPreferencesController < BaseController
      def show
        preference = policy_scope(NotificationPreference).find_by!(person_id: current_user.person_id)
        authorize preference

        render_resource(preference, serializer: NotificationPreferenceSerializer)
      end

      def update
        preference = current_user.person.notification_preference ||
                     current_user.person.build_notification_preference
        authorize preference

        return render_validation_errors(preference) unless preference.update(preference_params)

        render_resource(preference.reload, serializer: NotificationPreferenceSerializer)
      end

      private

      def preference_params
        params.expect(notification_preference: %i[
                        enabled morning_time afternoon_time evening_time night_time
                        dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled
                      ])
      end
    end
  end
end
