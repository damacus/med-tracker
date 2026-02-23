# frozen_string_literal: true

module FamilyDashboard
  # Query object to fetch a 24-hour medication schedule for a person and their dependents
  class ScheduleQuery
    def initialize(people)
      @people = Array(people)
    end

    def call
      preload_todays_takes
      # 1. Fetch all active prescriptions and person_medicines for these people
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
        t.prescription_id ? ['Prescription', t.prescription_id] : ['PersonMedicine', t.person_medicine_id]
      end
    end

    def fetch_todays_takes(person_ids)
      MedicationTake.where(taken_at: Time.current.all_day)
                    .where(prescription_id: Prescription.where(person_id: person_ids).select(:id))
                    .or(
                      MedicationTake.where(taken_at: Time.current.all_day)
                                    .where(person_medicine_id: PersonMedicine.where(person_id: person_ids).select(:id))
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

      # Prescriptions
      member.prescriptions.active.includes(:medicine, :dosage).find_each do |prescription|
        member_doses += generate_doses_for(prescription, member)
      end

      # PersonMedicines (Non-prescription)
      member.person_medicines.includes(:medicine).find_each do |pm|
        member_doses += generate_doses_for(pm, member)
      end

      member_doses
    end

    def generate_doses_for(source, person)
      # 1. Get doses already taken today from our preloaded cache
      takes = @takes_by_source[[source.class.name, source.id]] || []
      doses = generate_taken_doses(takes, source, person)

      # 2. Determine if an upcoming dose should be shown
      # We show the "next available" dose if it falls within today
      next_time = source.next_available_time
      if next_time && next_time.to_date == Time.zone.today
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
