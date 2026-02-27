# frozen_string_literal: true

# Controller for user profile management
class ProfilesController < ApplicationController
  before_action :require_authentication
  before_action :check_two_factor_setup

  def show
    @person = current_user.person
    @account = current_account

    respond_to do |format|
      format.html do
        render Views::Profiles::Show.new(person: @person, account: @account)
      end
    end
  end

  def update
    @person = current_user.person
    @account = current_account

    if person_params.present?
      if @person.update(person_params)
        respond_to do |format|
          format.html { redirect_to profile_path, notice: t('profiles.updated') }
          format.turbo_stream do
            flash.now[:notice] = t('profiles.updated')
            render turbo_stream: [
              turbo_stream.update('modal', ''),
              turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice])),
              turbo_stream.replace('main-content', Views::Profiles::Show.new(person: @person.reload, account: @account))
            ]
          end
        end
      else
        respond_to do |format|
          format.html { redirect_to profile_path, alert: t('profiles.profile_update_failed', errors: @person.errors.full_messages.join(', ')) }
          format.turbo_stream do
            flash.now[:alert] = t('profiles.profile_update_failed', errors: @person.errors.full_messages.join(', '))
            render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
          end
        end
      end
    elsif account_params.present?
      if @account.update(account_params)
        respond_to do |format|
          format.html { redirect_to profile_path, notice: t('profiles.email_updated') }
          format.turbo_stream do
            flash.now[:notice] = t('profiles.email_updated')
            render turbo_stream: [
              turbo_stream.update('modal', ''),
              turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice])),
              turbo_stream.replace('main-content', Views::Profiles::Show.new(person: @person, account: @account.reload))
            ]
          end
        end
      else
        respond_to do |format|
          format.html { redirect_to profile_path, alert: t('profiles.email_update_failed', errors: @account.errors.full_messages.join(', ')) }
          format.turbo_stream do
            flash.now[:alert] = t('profiles.email_update_failed', errors: @account.errors.full_messages.join(', '))
            render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
          end
        end
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
