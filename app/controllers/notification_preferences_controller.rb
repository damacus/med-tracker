# frozen_string_literal: true

class NotificationPreferencesController < ApplicationController
  before_action :check_two_factor_setup

  def edit
    @preference = current_user.person.notification_preference ||
                  current_user.person.build_notification_preference
    render Components::NotificationSettings::FormView.new(preference: @preference)
  end

  def update
    @preference = current_user.person.notification_preference ||
                  current_user.person.build_notification_preference
    if @preference.update(preference_params)
      redirect_to edit_notification_preference_path, notice: t('notification_preferences.updated')
    else
      render Components::NotificationSettings::FormView.new(preference: @preference), status: :unprocessable_content
    end
  end

  private

  def preference_params
    params.expect(notification_preference: %i[enabled morning_time afternoon_time evening_time night_time])
  end
end
