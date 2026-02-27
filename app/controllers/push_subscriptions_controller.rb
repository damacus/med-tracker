# frozen_string_literal: true

class PushSubscriptionsController < ApplicationController
  def create
    sub = current_account.push_subscriptions.find_or_initialize_by(endpoint: params[:endpoint])
    sub.assign_attributes(p256dh: params.dig(:keys, :p256dh), auth: params.dig(:keys, :auth),
                          user_agent: request.user_agent)
    sub.save!
    head :created
  end

  def destroy
    current_account.push_subscriptions.find_by(endpoint: params[:endpoint])&.destroy!
    head :no_content
  end
end
