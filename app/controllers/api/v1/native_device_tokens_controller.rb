# frozen_string_literal: true

module Api
  module V1
    class NativeDeviceTokensController < BaseController
      def create
        token = current_account.native_device_tokens.find_or_initialize_by(device_token: token_params[:device_token])
        token.assign_attributes(platform: token_params[:platform], user_agent: request.user_agent)
        authorize token, :create?

        return render_validation_errors(token) unless token.save

        head :created
      end

      def destroy
        token = current_account.native_device_tokens.find_by(device_token: params.expect(:id))
        unless token
          authorize NativeDeviceToken.new(account: current_account), :destroy?
          return head :no_content
        end

        authorize token, :destroy?
        token.destroy!
        head :no_content
      end

      private

      def token_params
        params.expect(native_device_token: %i[device_token platform])
      end
    end
  end
end
