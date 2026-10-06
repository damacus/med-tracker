module Api
  module V1
    class ProfilesController < BaseController
      rescue_from ActiveRecord::RecordInvalid do |error|
        render_validation_errors(error.record)
      end
      rescue_from Profiles::Update::InvalidAttributes do
        render_unprocessable('Profile attributes are invalid')
      end

      def show
        authorize profile_person, :show?
        render_profile
      end

      def update
        Profiles::Update.new(person: profile_person, account: current_account, authorization: pundit_user)
                        .call(profile_attributes,
                              supplied_keys: params[:profile].keys)
        render_profile
      end

      private

      def profile_attributes
        supplied = params.require(:profile)
        raise Profiles::Update::InvalidAttributes unless supplied.is_a?(ActionController::Parameters)

        supplied.permit(:date_of_birth, :time_zone, :gravatar_enabled, mobile_shortcuts: []).to_h
      end

      def with_api_idempotency(&)
        response.set_header('Cache-Control', 'no-store')
        authorize profile_person, :update? if action_name == 'update'
        super
      end

      def profile_person
        @profile_person ||= current_household.people.find_by!(id: current_membership.person_id,
                                                              account_id: current_account.id)
      end

      def render_profile
        render json: { data: ProfileSerializer.new(person: profile_person, account: current_account).as_json }
      end
    end
  end
end
