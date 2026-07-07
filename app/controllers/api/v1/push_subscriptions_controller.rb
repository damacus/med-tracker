# frozen_string_literal: true

module Api
  module V1
    class PushSubscriptionsController < BaseController
      def create
        subscription = current_account.push_subscriptions.find_or_initialize_by(endpoint: subscription_params[:endpoint])
        subscription.assign_attributes(
          p256dh: subscription_params.dig(:keys, :p256dh),
          auth: subscription_params.dig(:keys, :auth),
          user_agent: request.user_agent
        )
        authorize subscription, :create?

        return render_validation_errors(subscription) unless subscription.save

        head :created
      end

      def destroy
        subscription = current_account.push_subscriptions.find_by(endpoint: params.expect(:endpoint))
        unless subscription
          authorize PushSubscription.new(account: current_account), :destroy?
          return head :no_content
        end

        authorize subscription, :destroy?
        subscription.destroy!
        head :no_content
      end

      def test
        authorize PushSubscription.new(account: current_account), :test?
        PushNotificationService.send_to_account(
          current_account,
          title: 'MedTracker Test',
          body: 'Push notifications are working correctly from the server.'
        )
        head :no_content
      rescue StandardError
        render_api_error(
          code: 'push_test_failed',
          message: 'Unable to send test notification.',
          status: :service_unavailable
        )
      end

      private

      def subscription_params
        params.expect(push_subscription: [:endpoint, { keys: %i[p256dh auth] }])
      end
    end
  end
end
