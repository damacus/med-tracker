# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MissedDoseNotificationJob do
  fixtures :accounts, :people, :locations, :medications, :dosages, :carer_relationships

  let(:person) { people(:john) }
  let(:household) { person.household }
  let(:scheduled_on) { '2026-05-12' }
  let(:scheduled_time) { '07:15' }
  let(:events) { [] }

  before do
    PersonAccessGrant.where(household: household).delete_all
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
    allow(Observability::CanonicalLogger).to receive(:write) { |event| events << event.to_h }
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
      path: "/households/#{household.slug}/dashboard",
      notification_kind: :missed_dose
    )
    expect(NotificationEvent.where(event_type: 'missed_dose').count).to eq(1)
    expect(notification_reasons).to include('eligible', 'intent_recorded', 'attempted')
  end

  it 'suppresses duplicates for the same scheduled occurrence' do
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      2.times { described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time) }
    end

    expect(PushNotificationService).to have_received(:send_to_account).once
    expect(notification_reasons).to include('duplicate')
  end

  it 'keeps one workflow through evaluation, intent, channel attempt, and provider acceptance' do
    create_schedule
    allow(PushNotificationService).to receive(:send_to_account).and_call_original
    allow(WebPush).to receive(:payload_send)
    context = Observability::CorrelationContext.start.next_attempt
    Current.observability_context = context

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    notification_events = events.select { |event| event['event.name'] == 'notification.stage' }
    expect(notification_events.pluck('medtracker.reason')).to include(
      'eligible', 'intent_recorded', 'attempted', 'provider_accepted'
    )
    expect(notification_events.pluck('medtracker.workflow.id').uniq).to contain_exactly(context.workflow_id)
  ensure
    Current.observability_context = nil
  end

  it 'does not send when the dose was taken in the dose window' do
    schedule = create_schedule
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.local(2026, 5, 12, 7, 20))

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
    expect(notification_reasons).to include('no_due_dose')
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
    prepare_managed_children(parent: parent, children: children)
    perform_missed_dose_checks(*children)

    children.each { |child| expect_missed_dose_notification(parent, child) }
  end

  it 'does not disclose a managed person missed dose through a revoked grant' do
    manager = people(:jane)
    child = people(:child_patient)
    prepare_notification_recipient(manager)
    grant = prepare_managed_person(manager: manager, target: child, relationship_type: :parent)
    grant.update!(revoked_at: 1.minute.ago)
    perform_missed_dose_checks(child)

    expect_no_missed_dose_notification(manager, child)
  end

  it 'only notifies a manager about an adult after they opt in' do
    manager = people(:jane)
    managed_adult = people(:bob)
    prepare_notification_recipient(manager)
    grant = prepare_managed_person(manager: manager, target: managed_adult, relationship_type: :family_member)
    perform_missed_dose_checks(managed_adult)
    expect_no_missed_dose_notification(manager, managed_adult)

    grant.update!(missed_dose_notifications_enabled: true)
    perform_missed_dose_checks(managed_adult)
    expect_missed_dose_notification(manager, managed_adult)
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

  def prepare_managed_children(parent:, children:)
    relationship = CarerRelationship.find_or_initialize_by(
      household: household,
      carer: parent,
      patient: children.first
    )
    relationship.update!(relationship_type: 'parent', active: true)
    children.first.reload.update!(account: nil)
    children.each { |child| prepare_managed_person(manager: parent, target: child, relationship_type: :parent) }
  end

  def prepare_managed_person(manager:, target:, relationship_type:)
    clear_medication_activity(target)
    grant = grant_management_access(manager: manager, target: target, relationship_type: relationship_type)
    create_schedule(target)
    grant
  end

  def perform_missed_dose_checks(*targets)
    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      targets.each { |target| described_class.perform_now(household.id, target.id, scheduled_on, scheduled_time) }
    end
  end

  def expect_missed_dose_notification(manager, target)
    expect(PushNotificationService).to have_received(:send_to_account).with(
      manager.account,
      title: 'Medication reminder',
      body: "#{target.name} may have missed a dose.",
      path: "/households/#{household.slug}/dashboard",
      notification_kind: :missed_dose
    ).once
  end

  def expect_no_missed_dose_notification(manager, target)
    expect(PushNotificationService).not_to have_received(:send_to_account).with(
      manager.account,
      hash_including(body: "#{target.name} may have missed a dose.")
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

  def notification_reasons
    events.filter_map do |event|
      event['medtracker.reason'] if event['event.name'] == 'notification.stage'
    end
  end

  def clear_medication_activity(target)
    MedicationTake.where(schedule_id: target.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: target.person_medications.select(:id)).delete_all
    target.schedules.destroy_all
    target.person_medications.destroy_all
  end
end
