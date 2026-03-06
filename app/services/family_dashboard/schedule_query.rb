# frozen_string_literal: true

module FamilyDashboard
  # Query object to fetch a 24-hour medication schedule for a person and their dependents
  class ScheduleQuery
    attr_reader :current_user

    def initialize(people, current_user: nil)
      @people = Array(people)
      @current_user = current_user
    end

    def call
      # 1. Fetch all active schedules and person_medications for these people first
      # to get the IDs for take preloading
      @person_ids = @people.map(&:id)
      @all_schedules = fetch_active_schedules
      @all_person_medications = fetch_person_medications

      # 2. Preload takes using the specific IDs we just found
      preload_takes

      # 3. Aggregate all doses
      doses = aggregate_family_doses

      # 4. Sort by time and return
      doses.sort_by { |d| d[:scheduled_at] }
    end

    private

    def fetch_active_schedules
      @people.each_with_object({}) do |person, hash|
        schedules = if person.schedules.loaded?
                      person.schedules.select(&:active?)
                    else
                      person.schedules.active.includes(:medication, :dosage).to_a
                    end
        hash[person.id] = schedules
      end
    end

    def fetch_person_medications
      @people.each_with_object({}) do |person, hash|
        pms = if person.person_medications.loaded?
                person.person_medications.to_a
              else
                person.person_medications.includes(:medication).to_a
              end
        hash[person.id] = pms
      end
    end

    def preload_takes
      # Fetch takes for the last 30 days to cover weekly/monthly cycles
      # but focus on today for the dashboard
      all_takes = fetch_takes_for_sources

      # Group by [source_type, source_id] for fast lookup
      takes_by_source = all_takes.group_by do |t|
        t.schedule_id ? ['Schedule', t.schedule_id] : ['PersonMedication', t.person_medication_id]
      end

      # Associate preloaded takes with these objects to avoid N+1 in TimingRestrictions
      associate_takes_to_sources(all_sources, takes_by_source)
    end

    def all_sources
      @all_schedules.values.flatten + @all_person_medications.values.flatten
    end

    def fetch_takes_for_sources
      schedule_ids = @all_schedules.values.flatten.map(&:id)
      pm_ids = @all_person_medications.values.flatten.map(&:id)
      range = 30.days.ago..Time.current.end_of_day

      MedicationTake.where(taken_at: range, schedule_id: schedule_ids)
                    .or(MedicationTake.where(taken_at: range, person_medication_id: pm_ids))
                    .includes(:taken_from_location, :taken_from_medication)
                    .to_a
    end

    def associate_takes_to_sources(sources, takes_by_source)
      sources.each do |source|
        key = [source.class.name, source.id]
        takes = takes_by_source[key] || []

        # Set the association as loaded and assign the takes
        association = source.association(:medication_takes)
        association.loaded!
        association.target.concat(takes)
      end
    end

    def aggregate_family_doses
      @people.each_with_object([]) do |member, doses|
        (@all_schedules[member.id] || []).each do |schedule|
          doses.concat(generate_doses_for(schedule, member))
        end

        (@all_person_medications[member.id] || []).each do |pm|
          doses.concat(generate_doses_for(pm, member))
        end
      end
    end

    def generate_member_doses(member)
      self.class.new(member).call
    end

    def generate_doses_for(source, person)
      now = Time.current
      # 1. Get doses already taken today from our preloaded association
      # This uses the association preloaded in aggregate_family_doses to avoid N+1 queries
      takes = source.medication_takes.select { |t| Time.current.all_day.cover?(t.taken_at) }
      doses = generate_taken_doses(takes, source, person)

      # 2. Determine if an upcoming dose should be shown
      # We show the "next available" dose if it falls within today
      next_time = source.next_available_time
      if next_time && next_time <= now + 24.hours
        status = MedicationStockSourceResolver.new(user: current_user, source: source).blocked_reason || :upcoming
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
          status: :taken,
          taken_from_location_name: take.inventory_location&.name
        }
      end
    end
  end
end
