# frozen_string_literal: true

module FamilyDashboard
  class TakePreloader
    INCLUDES = [
      :taken_from_location,
      :taken_from_medication,
      { schedule: %i[person medication], person_medication: %i[person medication] }
    ].freeze

    def initialize(sources:, date:)
      @sources = sources
      @date = date
    end

    def call
      # Group by [source_type, source_id] for fast lookup
      takes_by_source = fetch_takes.group_by { |take| source_key_for_take(take) }
      # Associate preloaded takes with these objects to avoid N+1 in TimingRestrictions
      sources.each { |source| assign_takes(source, takes_by_source.fetch(source_key(source), [])) }
    end

    private

    attr_reader :sources, :date

    def fetch_takes
      scheduled = MedicationTake.where(taken_at: range, schedule_id: source_ids(Schedule))
      direct = MedicationTake.where(taken_at: range, person_medication_id: source_ids(PersonMedication))
      scheduled.or(direct).includes(*INCLUDES).to_a
    end

    def range
      (date.beginning_of_day - 30.days)..date.end_of_day
    end

    def source_ids(source_class)
      sources.filter_map { |source| source.id if source.is_a?(source_class) }
    end

    def source_key_for_take(take)
      take.schedule_id ? ['Schedule', take.schedule_id] : ['PersonMedication', take.person_medication_id]
    end

    def source_key(source)
      [source.class.name, source.id]
    end

    def assign_takes(source, takes)
      # Set the association as loaded and assign the takes
      association = source.association(:medication_takes)
      association.loaded!
      association.target.concat(takes)
    end
  end
end
