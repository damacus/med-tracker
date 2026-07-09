# frozen_string_literal: true

module Api
  module V1
    module Admin
      class SettingsController < BaseController
        before_action :require_fresh_privileged_action, only: :update

        def show
          render json: { data: household_payload(current_household) }
        end

        def update
          return render_validation_errors(current_household) unless current_household.update(household_params)

          audit_admin_action!(event_type: 'api/admin/household_settings/updated',
                              target: current_household,
                              outcome: 'success')
          render json: { data: household_payload(current_household) }
        end

        private

        def household_params
          params.expect(household: %i[name timezone subscription_plan])
        end

        def household_payload(household)
          {
            id: household.id,
            name: household.name,
            slug: household.slug,
            timezone: household.timezone,
            subscription_plan: household.subscription_plan,
            updated_at: household.updated_at.iso8601
          }
        end
      end
    end
  end
end
