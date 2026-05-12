# frozen_string_literal: true

class MedicationReminderJob < ApplicationJob
  queue_as :default

  PERIOD_LABELS = {
    morning: 'Morning',
    afternoon: 'Afternoon',
    evening: 'Evening',
    night: 'Night'
  }.freeze

  def perform(person_id, period, scheduled_time = nil)
    person = Person.find_by(id: person_id)
    return unless person
    return unless person.account

    med_names = active_medication_names(person, scheduled_time)
    return if med_names.empty?

    period_label = scheduled_time.presence || PERIOD_LABELS[period.to_sym] || period.to_s.humanize
    body = "#{period_label} medications: #{med_names.join(', ')}"

    PushNotificationService.send_to_account(
      person.account,
      title: 'Medication Reminder',
      body: body,
      path: '/'
    )
  end

  private

  def active_medication_names(person, scheduled_time)
    schedule_meds = active_schedules(person, scheduled_time).map(&:medication_name)
    person_meds = scheduled_time.present? ? [] : active_person_medication_names(person)
    (schedule_meds + person_meds).uniq
  end

  def active_schedules(person, scheduled_time)
    schedules = person.schedules.active.includes(:medication)
    return schedules if scheduled_time.blank?

    schedules.select do |schedule|
      Array(schedule.schedule_config.to_h['times']).compact_blank.include?(scheduled_time)
    end
  end

  def active_person_medication_names(person)
    person.person_medications.includes(:medication).map { |pm| pm.medication.display_name }
  end
end
