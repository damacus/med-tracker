# frozen_string_literal: true

class NativeDeviceTokensController < ApplicationController
  def create
    token = current_account.native_device_tokens.find_or_initialize_by(device_token: params[:device_token])
    token.assign_attributes(platform: params[:platform], user_agent: request.user_agent)

    if token.save
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
      head :no_content
    else
      render json: { error: 'Unable to remove device token.', errors: token.errors.full_messages },
             status: :unprocessable_content
    end
  end
end
