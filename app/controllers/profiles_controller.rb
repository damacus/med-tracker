# frozen_string_literal: true

# Controller for user profile management
class ProfilesController < ApplicationController
  before_action :require_authentication

  def show
    @person = current_user.person
    @account = current_account
  end

  def update
    @person = current_user.person
    @account = current_account

    if person_params.present?
      if @person.update(person_params)
        redirect_to profile_path, notice: 'Profile updated successfully.'
      else
        redirect_to profile_path, alert: "Failed to update profile: #{@person.errors.full_messages.join(', ')}"
      end
    elsif account_params.present?
      if @account.update(account_params)
        redirect_to profile_path, notice: 'Email updated successfully.'
      else
        redirect_to profile_path, alert: "Failed to update email: #{@account.errors.full_messages.join(', ')}"
      end
    else
      redirect_to profile_path, alert: 'No changes to save.'
    end
  end

  private

  def person_params
    params.expect(person: [:date_of_birth]) if params[:person]
  end

  def account_params
    params.expect(account: [:email]) if params[:account]
  end
end
