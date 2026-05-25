# frozen_string_literal: true

class MedicationReminderEligibilityQuery
  def initialize(person:, scheduled_time: nil, now: Time.current)
    @person = person
    @scheduled_time = scheduled_time.presence
    @now = now
  end

  def medication_names
    (due_schedule_names + due_person_medication_names).uniq
  end

  def configured_times
    due_schedules.flat_map { |schedule| due_configured_times_for(schedule) }.uniq
  end

  private

  attr_reader :person, :scheduled_time, :now

  def due_schedule_names
    due_schedules.map(&:medication_name)
  end

  def due_person_medication_names
    return [] if scheduled_time.present?

    person_medications.select { |person_medication| due_person_medication?(person_medication) }
                      .map { |person_medication| person_medication.medication.display_name }
  end

  def due_schedules
    @due_schedules ||= schedules.select { |schedule| due_schedule?(schedule) }
  end

  def schedules
    @schedules ||= person.schedules.active.includes(:medication, :medication_takes).to_a
  end

  def person_medications
    @person_medications ||= person.person_medications.routine.includes(:medication, :medication_takes).to_a
  end

  def due_schedule?(schedule)
    return false if as_needed_schedule?(schedule)
    return false if configured_times_for(schedule).blank?
    return false if taken_in_current_cycle?(schedule)
    return false unless schedule.applies_on?(today)

    if scheduled_time.present?
      scheduled_occurrence_due?(schedule)
    else
      remaining_schedule_doses?(schedule)
    end
  end

  def due_person_medication?(person_medication)
    return false if taken_in_current_cycle?(person_medication)

    taken_count_for_cycle(person_medication) < expected_person_medication_doses(person_medication)
  end

  def scheduled_occurrence_due?(schedule)
    configured_times_for(schedule).each_with_index.any? do |time, index|
      time == scheduled_time && taken_count_for_cycle(schedule) <= index
    end
  end

  def due_configured_times_for(schedule)
    configured_times_for(schedule).filter.with_index do |_time, index|
      taken_count_for_cycle(schedule) <= index
    end
  end

  def remaining_schedule_doses?(schedule)
    expected = schedule.expected_doses_on(today)
    expected.positive? && taken_count_for_cycle(schedule) < expected
  end

  def expected_person_medication_doses(person_medication)
    person_medication.max_daily_doses.presence || 1
  end

  def taken_count_for_cycle(source)
    cycle = DoseCycle.new(source.respond_to?(:dose_cycle) ? source.dose_cycle : 'daily')
    medication_takes_for(source).count { |take| cycle.range_for(now).cover?(take.taken_at) }
  end

  def taken_in_current_cycle?(source)
    taken_count_for_cycle(source).positive?
  end

  def medication_takes_for(source)
    takes_by_medication_id.fetch(source.medication_id, [])
  end

  def takes_by_medication_id
    @takes_by_medication_id ||= begin
      medication_ids = reminder_source_medication_ids

      if medication_ids.blank?
        {}
      else
        (schedule_takes_for(medication_ids) + person_medication_takes_for(medication_ids)).group_by do |take|
          take.schedule&.medication_id || take.person_medication&.medication_id
        end
      end
    end
  end

  def reminder_source_medication_ids
    (schedules + person_medications).map(&:medication_id).compact.uniq
  end

  def schedule_takes_for(medication_ids)
    MedicationTake.joins(:schedule)
                  .where(taken_at: medication_take_lookup_range,
                         schedules: { person_id: person.id, medication_id: medication_ids })
                  .includes(:schedule)
                  .to_a
  end

  def person_medication_takes_for(medication_ids)
    MedicationTake.joins(:person_medication)
                  .where(taken_at: medication_take_lookup_range,
                         person_medications: { person_id: person.id, medication_id: medication_ids })
                  .includes(:person_medication)
                  .to_a
  end

  def medication_take_lookup_range
    (now - 1.month).beginning_of_day..now.end_of_day
  end

  def as_needed_schedule?(schedule)
    schedule.schedule_type_prn? ||
      schedule_config_value(schedule, 'as_needed') == true ||
      schedule.frequency.to_s.casecmp('as needed').zero?
  end

  def configured_times_for(schedule)
    Array(schedule_config_value(schedule, 'times')).compact_blank
  end

  def schedule_config_value(schedule, key)
    config = schedule.schedule_config.to_h
    config[key] || config[key.to_sym]
  end

  def today
    now.to_date
  end
end
