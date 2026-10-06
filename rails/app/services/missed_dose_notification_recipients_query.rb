# frozen_string_literal: true

class MissedDoseNotificationRecipientsQuery
  Recipient = Data.define(:account, :preference, :managed)

  def initialize(person:)
    @person = person
  end

  def call
    [self_recipient, *managed_recipients].compact.uniq { |recipient| recipient.account.id }
  end

  private

  attr_reader :person

  def self_recipient
    return unless person.account && eligible_preference?(person.notification_preference)

    Recipient.new(account: person.account, preference: person.notification_preference, managed: false)
  end

  def managed_recipients
    managed_grants.filter_map do |grant|
      next unless grant.missed_dose_notifications_included?

      membership = grant.household_membership
      preference = membership.person&.notification_preference
      next unless eligible_preference?(preference)

      Recipient.new(account: membership.account, preference: preference, managed: true)
    end
  end

  def managed_grants
    PersonAccessGrant.active.manage
                     .where(household: person.household, person: person)
                     .joins(:household_membership)
                     .merge(HouseholdMembership.active)
                     .includes(household_membership: [:account, { person: :notification_preference }])
  end

  def eligible_preference?(preference)
    preference&.enabled && preference.missed_dose_enabled
  end
end
