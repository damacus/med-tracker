# frozen_string_literal: true

# Controller for user profile management
class ProfilesController < ApplicationController
  PROFILE_SETTING_TARGETS = {
    'avatar' => 'profile-avatar-errors',
    'time_zone' => 'profile-time-zone-errors'
  }.freeze

  before_action :require_authentication
  before_action :check_two_factor_setup

  def show
    @person = current_user.person
    @account = current_account
    authorize @person, :show?

    respond_to do |format|
      format.html do
        render profile_view(person: @person, account: @account, active_section: params[:section])
      end
    end
  end

  def update
    @person = current_user.person
    @account = current_account
    authorize @person, :update?

    attributes = person_params
    return update_person_profile(attributes) if attributes.present?
    return respond_email_change_requires_verification if direct_email_change_requested?

    attributes = account_params
    return update_account_profile(attributes) if attributes.present?

    respond_no_changes
  end

  def avatar
    authorize current_user.person, :update?
    current_user.person.avatar.purge
    @person = current_user.person
    @account = current_account

    respond_to do |format|
      format.html { redirect_to profile_redirect_path, notice: t('.removed') }
      format.turbo_stream do
        flash.now[:notice] = t('.removed')
        render turbo_stream: [
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice])),
          turbo_stream.replace(
            'main-content',
            profile_view(person: @person, account: @account, active_section: profile_section)
          )
        ]
      end
    end
  end

  def experiments
    authorize current_user.person, :update?

    if current_account.update(experiment_preferences)
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: [
            turbo_stream.replace(
              'experiments-card',
              Views::Profiles::ExperimentsCard.new(account: current_account)
            )
          ]
        end
        format.html { redirect_to profile_redirect_path }
      end
    else
      respond_to do |format|
        format.turbo_stream { head :unprocessable_content }
        format.html do
          redirect_to profile_redirect_path,
                      alert: 'Could not save preference.' # rubocop:disable Rails/I18nLocaleTexts
        end
      end
    end
  end

  private

  def profile_view(person:, account:, active_section: nil, new_api_app_token: nil)
    Views::Profiles::Show.new(
      presenter: Profiles::Presenter.build(
        person:,
        account:,
        household: person.household,
        active_section:,
        new_api_app_token:
      )
    )
  end

  def experiment_preferences
    preferences = params.fetch(:account, {}).permit(
      :wizard_variant,
      :dashboard_variant,
      :medication_launcher_variant
    )

    {}.tap do |attributes|
      if preferences.key?(:wizard_variant)
        attributes[:wizard_variant] = normalized_variant(
          preferences[:wizard_variant],
          allowed: Account::WIZARD_VARIANTS,
          fallback: 'fullpage'
        )
      end
      if preferences.key?(:dashboard_variant)
        attributes[:dashboard_variant] = normalized_variant(
          preferences[:dashboard_variant],
          allowed: Account::DASHBOARD_VARIANTS,
          fallback: 'current'
        )
      end
      if preferences.key?(:medication_launcher_variant)
        attributes[:medication_launcher_variant] = normalized_variant(
          preferences[:medication_launcher_variant],
          allowed: Account::MEDICATION_LAUNCHER_VARIANTS,
          fallback: 'current'
        )
      end
    end
  end

  def normalized_variant(value, allowed:, fallback:)
    variant = value.to_s
    allowed.include?(variant) ? variant : fallback
  end

  def update_person_profile(attributes)
    if @person.update(attributes)
      respond_profile_updated(person: @person.reload, account: @account, close_modal: true)
    else
      respond_profile_failed(@person)
    end
  end

  def update_account_profile(attributes)
    if @account.update(attributes)
      respond_profile_updated(person: @person, account: @account)
    else
      respond_profile_failed(@account)
    end
  end

  def respond_profile_updated(person:, account:, close_modal: false)
    respond_to do |format|
      format.html { redirect_to profile_redirect_path, notice: t('profiles.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('profiles.updated')
        streams = [turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))]
        streams.unshift(turbo_stream.update('modal', '')) if close_modal
        streams << turbo_stream.replace(
          'main-content',
          profile_view(person:, account:, active_section: profile_section)
        )
        render turbo_stream: streams
      end
    end
  end

  def respond_profile_failed(record)
    message = t('profiles.profile_update_failed', errors: record.errors.full_messages.join(', '))
    respond_to do |format|
      format.html { redirect_to profile_redirect_path, alert: message }
      format.turbo_stream do
        flash.now[:alert] = message
        options = { turbo_stream: profile_error_stream(message) }
        options[:status] = :unprocessable_content if profile_error_target
        render(**options)
      end
    end
  end

  def respond_no_changes
    respond_to do |format|
      format.html { redirect_to profile_redirect_path, alert: t('profiles.no_changes') }
      format.turbo_stream do
        flash.now[:alert] = t('profiles.no_changes')
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def respond_email_change_requires_verification
    message = t('profiles.email_change_requires_verification')
    respond_to do |format|
      format.html { redirect_to '/change-login', alert: message }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
               status: :unprocessable_content
      end
    end
  end

  def person_params
    params.expect(person: %i[date_of_birth avatar]) if params[:person]
  end

  def account_params
    params.expect(account: %i[gravatar_enabled time_zone]) if params[:account]
  end

  def direct_email_change_requested?
    account_attributes = params[:account]
    account_attributes.respond_to?(:key?) && (account_attributes.key?(:email) || account_attributes.key?('email'))
  end

  def profile_section
    requested = params[:section].to_s
    Profiles::Presenter::SECTIONS.include?(requested) ? requested : 'profile'
  end

  def profile_redirect_path
    requested = params[:section].to_s
    return profile_path unless Profiles::Presenter::SECTIONS.include?(requested)

    profile_path(section: requested)
  end

  def profile_error_stream(message)
    target = profile_error_target
    return turbo_stream.update('flash', Components::Layouts::Flash.new(alert: message)) unless target

    turbo_stream.update(target, RubyUI::Alert.new(variant: :destructive) { message })
  end

  def profile_error_target
    PROFILE_SETTING_TARGETS[params[:profile_setting].to_s]
  end
end
