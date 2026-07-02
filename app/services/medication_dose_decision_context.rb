# frozen_string_literal: true

class MedicationDoseDecisionContext
  Decision = Data.define(:blocking_source, :source_count)

  def initialize(source:, taken_at:)
    @source = source
    @taken_at = taken_at
  end

  def blocked?
    decision.blocking_source.present?
  end

  def blocked_reason
    :overlapping_prescription_restriction if blocked?
  end

  def audit_payload
    {
      decision_source_count: decision.source_count,
      decision_blocking_source_type: source_type(decision.blocking_source),
      decision_blocking_source_id: decision.blocking_source&.id
    }.compact
  end

  private

  attr_reader :source, :taken_at

  def decision
    @decision ||= Decision.new(
      blocking_source: related_sources.find { |candidate| blocks_take?(candidate) },
      source_count: related_sources.size
    )
  end

  def blocks_take?(candidate)
    return false unless candidate.restrictions?

    !DoseTimingPolicy.new(
      takes: related_takes,
      dose_constraints: candidate.dose_constraints,
      dose_cycle: candidate.respond_to?(:dose_cycle) ? candidate.dose_cycle : 'daily'
    ).can_take_at?(taken_at)
  end

  def related_sources
    @related_sources ||= (related_schedules + related_person_medications).sort_by do |candidate|
      [source_sort_order(candidate), candidate.id]
    end
  end

  def related_schedules
    Schedule.where(person_id: person_id, medication_id: medication_id, active: true)
            .where('start_date <= ? AND end_date >= ?', effective_date, effective_date)
            .to_a
  end

  def related_person_medications
    PersonMedication.where(person_id: person_id, medication_id: medication_id, active: true).to_a
  end

  def related_takes
    @related_takes ||=
      MedicationTake
      .where(schedule_id: schedule_ids)
      .or(MedicationTake.where(person_medication_id: person_medication_ids))
      .where(taken_at: 31.days.ago.beginning_of_day..Time.current.end_of_day)
      .to_a
  end

  def schedule_ids
    related_sources.grep(Schedule).map(&:id)
  end

  def person_medication_ids
    related_sources.grep(PersonMedication).map(&:id)
  end

  def source_sort_order(candidate)
    return 0 if candidate.is_a?(Schedule)
    return 1 if candidate.is_a?(PersonMedication)

    2
  end

  def source_type(candidate)
    return unless candidate

    candidate.class.model_name.singular
  end

  def person_id
    source.person_id
  end

  def medication_id
    source.medication_id
  end

  def effective_date
    return taken_at.to_date if taken_at.respond_to?(:to_date)

    Time.zone.today
  end
end
