# frozen_string_literal: true

module FamilyDashboard
  # Query object to fetch a 24-hour medication schedule for a person and their dependents
  class ScheduleQuery
    def initialize(people)
      @people = Array(people)
    end

    def call
      preload_todays_takes
      # 1. Fetch all active schedules and person_medications for these people
      doses = aggregate_family_doses(@people)

      # 2. Sort by time and return
      doses.sort_by { |d| d[:scheduled_at] }
    end

    private

    def preload_todays_takes
      person_ids = @people.map(&:id)

      all_takes = fetch_todays_takes(person_ids)

      # Group by [source_type, source_id] for fast lookup
      @takes_by_source = all_takes.group_by do |t|
        t.schedule_id ? ['Schedule', t.schedule_id] : ['PersonMedication', t.person_medication_id]
      end
    end

    def fetch_todays_takes(person_ids)
      MedicationTake.where(taken_at: Time.current.all_day)
                    .where(schedule_id: Schedule.where(person_id: person_ids).select(:id))
                    .or(
                      MedicationTake.where(taken_at: Time.current.all_day)
                                    .where(
                                      person_medication_id: PersonMedication.where(person_id: person_ids).select(:id)
                                    )
                    )
                    .to_a
    end

    def aggregate_family_doses(family_members)
      family_members.each_with_object([]) do |member, doses|
        doses.concat(generate_member_doses(member))
      end
    end

    def generate_member_doses(member)
      member_doses = []

      # Schedules
      member.schedules.active.includes(:medication, :dosage).find_each do |schedule|
        member_doses += generate_doses_for(schedule, member)
      end

      # PersonMedications (Non-schedule)
      member.person_medications.includes(:medication).find_each do |pm|
        member_doses += generate_doses_for(pm, member)
      end

      member_doses
    end

    def generate_doses_for(source, person)
      now = Time.current
      # 1. Get doses already taken today from our preloaded cache
      takes = @takes_by_source[[source.class.name, source.id]] || []
      doses = generate_taken_doses(takes, source, person)

      # 2. Determine if an upcoming dose should be shown
      # We show the "next available" dose if it falls within today
      next_time = source.next_available_time
      if next_time && next_time <= now + 24.hours
        status = source.administration_blocked_reason || :upcoming
        doses << {
          person: person,
          source: source,
          scheduled_at: next_time,
          taken_at: nil,
          status: status
        }
      end

      doses
    end

    def generate_taken_doses(takes, source, person)
      takes.map do |take|
        {
          person: person,
          source: source,
          scheduled_at: take.taken_at,
          taken_at: take.taken_at,
          status: :taken
        }
      end
    end
  end
end
