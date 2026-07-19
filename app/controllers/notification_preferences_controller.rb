# frozen_string_literal: true

class NotificationPreferencesController < ApplicationController
  def update
    @preference = current_user.person.notification_preference ||
                  current_user.person.build_notification_preference
    authorize @preference
    attributes = preference_params
    managed_person_ids = attributes.delete(:managed_person_ids)
    updater = NotificationPreferenceUpdater.new(
      preference: @preference,
      membership: current_membership,
      preference_attributes: attributes,
      managed_person_ids: managed_person_ids
    )

    if updater.call
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
    params.expect(notification_preference: [
                    :enabled, :morning_time, :afternoon_time, :evening_time, :night_time,
                    :dose_due_enabled, :missed_dose_enabled, :low_stock_enabled, :private_text_enabled,
                    { managed_person_ids: [] }
                  ])
  end

  def notification_preference_streams
    [
      turbo_stream.replace(
        'notifications-card',
        Views::Profiles::NotificationsCard.new(
          person: current_user.person.reload,
          managed_grants: managed_notification_grants
        )
      ),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def current_membership
    @current_membership ||= current_account.active_household_membership_for(current_household)
  end

  def managed_notification_grants
    ManagedNotificationGrantsQuery.new(membership: current_membership).call
  end
end
