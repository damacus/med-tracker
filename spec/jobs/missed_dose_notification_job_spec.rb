# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MissedDoseNotificationJob do
  fixtures :accounts, :people, :locations, :medications, :dosages, :carer_relationships

  let(:person) { people(:john) }
  let(:household) { person.household }
  let(:scheduled_on) { '2026-05-12' }
  let(:scheduled_time) { '07:15' }

  before do
    MedicationTake.where(schedule_id: person.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: person.person_medications.select(:id)).delete_all
    person.schedules.destroy_all
    person.person_medications.destroy_all
    PushSubscription.create!(
      account: person.account,
      endpoint: 'https://fcm.googleapis.com/fcm/send/missed-dose',
      p256dh: 'public-key',
      auth: 'auth-secret'
    )
    person.create_notification_preference!(enabled: true, missed_dose_enabled: true)
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends one private notification when a scheduled dose is overdue' do
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication reminder',
      body: 'A dose may have been missed.',
      path: "/households/#{household.slug}/dashboard"
    )
    expect(NotificationEvent.where(event_type: 'missed_dose').count).to eq(1)
  end

  it 'suppresses duplicates for the same scheduled occurrence' do
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      2.times { described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time) }
    end

    expect(PushNotificationService).to have_received(:send_to_account).once
  end

  it 'does not send when the dose was taken in the dose window' do
    schedule = create_schedule
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.local(2026, 5, 12, 7, 20))

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send when missed-dose notifications are disabled' do
    person.notification_preference.update!(missed_dose_enabled: false)
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'records a skip when the account has no active push subscriptions' do
    person.account.push_subscriptions.destroy_all
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    event = NotificationEvent.find_by!(event_type: 'missed_dose')
    expect(event.skipped_reason).to eq('no_active_push_subscriptions')
    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'notifies a parent once for each managed child who misses a dose' do
    parent = people(:jane)
    children = [people(:child_patient), people(:child_user_person)]
    prepare_notification_recipient(parent)
    relationship = CarerRelationship.find_or_initialize_by(household: household, carer: parent,
                                                            patient: children.first)
    relationship.update!(relationship_type: 'parent', active: true)
    children.first.reload.update!(account: nil)

    children.each do |child|
      clear_medication_activity(child)
      grant_management_access(manager: parent, target: child, relationship_type: :parent)
      create_schedule(child)
    end

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      children.each do |child|
        described_class.perform_now(household.id, child.id, scheduled_on, scheduled_time)
      end
    end

    children.each do |child|
      expect(PushNotificationService).to have_received(:send_to_account).with(
        parent.account,
        title: 'Medication reminder',
        body: "#{child.name} may have missed a dose.",
        path: "/households/#{household.slug}/dashboard"
      ).once
    end
  end

  it 'does not disclose a managed person missed dose through a revoked grant' do
    manager = people(:jane)
    child = people(:child_patient)
    prepare_notification_recipient(manager)
    clear_medication_activity(child)
    grant = grant_management_access(manager: manager, target: child, relationship_type: :parent)
    grant.update!(revoked_at: 1.minute.ago)
    create_schedule(child)

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, child.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account).with(
      manager.account,
      hash_including(body: "#{child.name} may have missed a dose.")
    )
  end

  it 'only notifies a manager about an adult after they opt in' do
    manager = people(:jane)
    managed_adult = people(:bob)
    prepare_notification_recipient(manager)
    clear_medication_activity(managed_adult)
    grant = grant_management_access(manager: manager, target: managed_adult,
                                    relationship_type: :family_member)
    create_schedule(managed_adult)

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, managed_adult.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account).with(
      manager.account,
      hash_including(body: "#{managed_adult.name} may have missed a dose.")
    )

    grant.update!(missed_dose_notifications_enabled: true)

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, managed_adult.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).to have_received(:send_to_account).with(
      manager.account,
      title: 'Medication reminder',
      body: "#{managed_adult.name} may have missed a dose.",
      path: "/households/#{household.slug}/dashboard"
    ).once
  end

  def create_schedule(target = person)
    create(:schedule, person: target, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily,
                      schedule_config: { 'times' => [scheduled_time] },
                      start_date: Date.parse(scheduled_on) - 1.day, end_date: Date.parse(scheduled_on) + 1.month)
  end

  def prepare_notification_recipient(recipient)
    recipient.notification_preference&.destroy!
    recipient.create_notification_preference!(enabled: true, missed_dose_enabled: true)
    PushSubscription.create!(
      account: recipient.account,
      endpoint: "https://fcm.googleapis.com/fcm/send/#{recipient.id}",
      p256dh: 'public-key',
      auth: 'auth-secret'
    )
  end

  def grant_management_access(manager:, target:, relationship_type:)
    membership = household.household_memberships.find_or_create_by!(account: manager.account) do |record|
      record.person = manager
      record.role = :member
      record.status = :active
    end
    PersonAccessGrant.create!(
      household: household,
      household_membership: membership,
      person: target,
      access_level: :manage,
      relationship_type: relationship_type,
      granted_by_membership: membership
    )
  end

  def clear_medication_activity(target)
    MedicationTake.where(schedule_id: target.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: target.person_medications.select(:id)).delete_all
    target.schedules.destroy_all
    target.person_medications.destroy_all
  end
end
