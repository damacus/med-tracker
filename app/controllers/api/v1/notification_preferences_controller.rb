# frozen_string_literal: true

module Api
  module V1
    class NotificationPreferencesController < BaseController
      def show
        preference = policy_scope(NotificationPreference).find_by!(person_id: current_user.person_id)
        authorize preference

        render_resource(preference, serializer: NotificationPreferenceSerializer)
      end
    end
  end
end
