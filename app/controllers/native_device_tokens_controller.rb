# frozen_string_literal: true

class NativeDeviceTokensController < ApplicationController
  def create
    token = current_account.native_device_tokens.find_or_initialize_by(device_token: params[:device_token])
    new_token = token.new_record?
    token.assign_attributes(platform: params[:platform], user_agent: request.user_agent)

    if token.save
      record_auth_token_event('created', token) if new_token
      head :created
    else
      render json: { error: 'Unable to save device token.', errors: token.errors.full_messages },
             status: :unprocessable_content
    end
  end

  def destroy
    token = current_account.native_device_tokens.find_by(device_token: params[:id])
    return head :no_content unless token

    if token.destroy
      record_auth_token_event('revoked', token)
      head :no_content
    else
      render json: { error: 'Unable to remove device token.', errors: token.errors.full_messages },
             status: :unprocessable_content
    end
  end

  private

  def record_auth_token_event(action, token)
    AuthTokenAuditLogger.new.record(
      account: current_account,
      token_type: 'native_device_token',
      action: action,
      metadata: {
        user_agent: token.user_agent,
        platform: token.platform
      }
    )
  end
end
