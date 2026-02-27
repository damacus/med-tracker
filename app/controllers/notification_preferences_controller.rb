# frozen_string_literal: true

class NotificationPreferencesController < ApplicationController
  def update
    @preference = current_user.person.notification_preference ||
                  current_user.person.build_notification_preference
    if @preference.update(preference_params)
      redirect_to profile_path, notice: t('notification_preferences.updated')
    else
      redirect_to profile_path, alert: t('notification_preferences.update_failed')
    end
  end

  private

  def preference_params
    params.expect(notification_preference: %i[enabled morning_time afternoon_time evening_time night_time])
  end
end
