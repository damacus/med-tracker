# frozen_string_literal: true

class ManagedMissedDoseNotificationSubjectsQuery
  def initialize(household:)
    @household = household
  end

  def call
    eligible_grants.filter_map do |grant|
      next unless grant.missed_dose_notifications_included?
      next unless eligible_preference?(grant.household_membership.person&.notification_preference)

      grant.person
    end.uniq(&:id)
  end

  private

  attr_reader :household

  def eligible_grants
    PersonAccessGrant.active.manage
                     .where(household: household)
                     .joins(:household_membership)
                     .merge(HouseholdMembership.active)
                     .includes(
                       person: { schedules: %i[medication medication_takes] },
                       household_membership: { person: :notification_preference }
                     )
  end

  def eligible_preference?(preference)
    preference&.enabled && preference.missed_dose_enabled
  end
end
