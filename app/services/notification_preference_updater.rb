# frozen_string_literal: true

class NotificationPreferenceUpdater
  def initialize(preference:, membership:, preference_attributes:, managed_person_ids: nil)
    @preference = preference
    @membership = membership
    @preference_attributes = preference_attributes
    @managed_person_ids = managed_person_ids
  end

  def call
    updated = false

    NotificationPreference.transaction do
      raise ActiveRecord::Rollback unless preference.update(preference_attributes)

      update_managed_adult_grants if managed_person_ids
      updated = true
    end

    updated
  rescue ActiveRecord::RecordInvalid
    false
  end

  private

  attr_reader :preference, :membership, :preference_attributes, :managed_person_ids

  def update_managed_adult_grants
    selected_ids = Array(managed_person_ids).filter_map { |id| Integer(id, exception: false) }

    ManagedNotificationGrantsQuery.new(membership: membership).call.each do |grant|
      next unless grant.person.adult?

      enabled = selected_ids.include?(grant.person_id)
      grant.update!(missed_dose_notifications_enabled: enabled) if grant.missed_dose_notifications_enabled? != enabled
    end
  end
end
