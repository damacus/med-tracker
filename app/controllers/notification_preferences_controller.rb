# frozen_string_literal: true

class NotificationPreferencesController < ApplicationController
  def update
    @preference = current_user.person.notification_preference ||
                  current_user.person.build_notification_preference
    authorize @preference

    if @preference.update(preference_params)
      respond_to do |format|
        format.html { redirect_to profile_path, notice: t('notification_preferences.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('notification_preferences.updated')
          render turbo_stream: notification_preference_streams
        end
      end
    else
      respond_to do |format|
        format.html { redirect_to profile_path, alert: t('notification_preferences.update_failed') }
        format.turbo_stream do
          flash.now[:alert] = t('notification_preferences.update_failed')
          render turbo_stream: notification_preference_streams, status: :unprocessable_content
        end
      end
    end
  end

  private

  def preference_params
    params.expect(notification_preference: %i[
                    enabled morning_time afternoon_time evening_time night_time
                    dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled
                  ])
  end

  def notification_preference_streams
    [
      turbo_stream.replace('notifications-card', Views::Profiles::NotificationsCard.new(person: current_user.person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end
end
