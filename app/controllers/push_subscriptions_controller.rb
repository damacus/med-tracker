# frozen_string_literal: true

class PushSubscriptionsController < ApplicationController
  def create
    sub = current_account.push_subscriptions.find_or_initialize_by(endpoint: params[:endpoint])
    sub.assign_attributes(p256dh: params.dig(:keys, :p256dh), auth: params.dig(:keys, :auth),
                          user_agent: request.user_agent)
    if sub.save
      head :created
    else
      render json: { error: 'Unable to save push subscription.', errors: sub.errors.full_messages },
             status: :unprocessable_content
    end
  end

  def destroy
    sub = current_account.push_subscriptions.find_by(endpoint: params[:endpoint])
    return head :no_content unless sub

    if sub.destroy
      head :no_content
    else
      render json: { error: 'Unable to save push subscription.', errors: sub.errors.full_messages },
             status: :unprocessable_content
    end
  end

  def test
    PushNotificationService.send_to_account(
      current_account,
      title: 'MedTracker Test',
      body: 'Push notifications are working correctly from the server.'
    )
    head :no_content
  end
end
