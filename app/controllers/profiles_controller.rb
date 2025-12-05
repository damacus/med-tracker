# frozen_string_literal: true

# Controller for user profile management
class ProfilesController < ApplicationController
  before_action :require_authentication

  def show
    @person = current_user.person
    @account = current_account
    render Views::Profiles::Show.new(person: @person, account: @account)
  end

  def update
    @person = current_user.person
    @account = current_account

    if person_params.present?
      if @person.update(person_params)
        redirect_to profile_path, notice: t('profiles.updated')
      else
        redirect_to profile_path, alert: t('profiles.profile_update_failed', errors: @person.errors.full_messages.join(', '))
      end
    elsif account_params.present?
      if @account.update(account_params)
        redirect_to profile_path, notice: t('profiles.email_updated')
      else
        redirect_to profile_path, alert: t('profiles.email_update_failed', errors: @account.errors.full_messages.join(', '))
      end
    else
      redirect_to profile_path, alert: t('profiles.no_changes')
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
