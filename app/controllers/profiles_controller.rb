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
        render Views::Profiles::Show.new(person: @person, account: @account, user: current_user)
      end
    end
  end

  def update
    @person = current_user.person
    @account = current_account

    attributes = person_params
    return update_person_profile(attributes) if attributes.present?

    attributes = user_params
    return update_user_profile(attributes) if attributes.present?

    attributes = account_params
    return update_account_profile(attributes) if attributes.present?

    respond_no_changes
  end

  def avatar
    current_user.person.avatar.purge
    @person = current_user.person
    @account = current_account

    respond_to do |format|
      format.html { redirect_to profile_path, notice: t('.removed') }
      format.turbo_stream do
        flash.now[:notice] = t('.removed')
        render turbo_stream: [
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice])),
          turbo_stream.replace('main-content', Views::Profiles::Show.new(person: @person, account: @account, user: current_user))
        ]
      end
    end
  end

  def experiments
    variant = params.dig(:user, :wizard_variant).to_s
    variant = 'fullpage' unless User::WIZARD_VARIANTS.include?(variant)

    if current_user.update(wizard_variant: variant)
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: [
            turbo_stream.replace(
              'experiments-card',
              Views::Profiles::ExperimentsCard.new(user: current_user)
            )
          ]
        end
        format.html { redirect_to profile_path }
      end
    else
      respond_to do |format|
        format.turbo_stream { head :unprocessable_content }
        format.html { redirect_to profile_path, alert: 'Could not save preference.' } # rubocop:disable Rails/I18nLocaleTexts
      end
    end
  end

  private

  def update_person_profile(attributes)
    if @person.update(attributes)
      respond_profile_updated(person: @person.reload, account: @account, close_modal: true)
    else
      respond_profile_failed(@person)
    end
  end

  def update_user_profile(attributes)
    if current_user.update(attributes)
      respond_profile_updated(person: @person, account: @account)
    else
      respond_profile_failed(current_user)
    end
  end

  def update_account_profile(attributes)
    if @account.update(attributes)
      respond_email_updated
    else
      respond_email_failed
    end
  end

  def respond_profile_updated(person:, account:, close_modal: false)
    respond_to do |format|
      format.html { redirect_to profile_path, notice: t('profiles.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('profiles.updated')
        streams = [turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))]
        streams.unshift(turbo_stream.update('modal', '')) if close_modal
        streams << turbo_stream.replace('main-content', Views::Profiles::Show.new(person: person, account: account, user: current_user))
        render turbo_stream: streams
      end
    end
  end

  def respond_profile_failed(record)
    message = t('profiles.profile_update_failed', errors: record.errors.full_messages.join(', '))
    respond_to do |format|
      format.html { redirect_to profile_path, alert: message }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def respond_email_updated
    respond_to do |format|
      format.html { redirect_to profile_path, notice: t('profiles.email_updated') }
      format.turbo_stream do
        flash.now[:notice] = t('profiles.email_updated')
        render turbo_stream: [
          turbo_stream.update('modal', ''),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice])),
          turbo_stream.replace('main-content', Views::Profiles::Show.new(person: @person, account: @account.reload, user: current_user))
        ]
      end
    end
  end

  def respond_email_failed
    message = t('profiles.email_update_failed', errors: @account.errors.full_messages.join(', '))
    respond_to do |format|
      format.html { redirect_to profile_path, alert: message }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def respond_no_changes
    respond_to do |format|
      format.html { redirect_to profile_path, alert: t('profiles.no_changes') }
      format.turbo_stream do
        flash.now[:alert] = t('profiles.no_changes')
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def person_params
    params.expect(person: %i[date_of_birth avatar]) if params[:person]
  end

  def account_params
    params.expect(account: [:email]) if params[:account]
  end

  def user_params
    params.expect(user: [:gravatar_enabled]) if params[:user]
  end
end
