# frozen_string_literal: true

class MedicationInteractionLookup
  Interaction = Data.define(:candidate_terms, :existing_terms, :severity, :description)

  RULES = [
    Interaction.new(
      candidate_terms: %w[warfarin],
      existing_terms: %w[aspirin ibuprofen naproxen diclofenac],
      severity: 'high',
      description: 'May increase bleeding risk when used together. Review before adding or prescribing.'
    ),
    Interaction.new(
      candidate_terms: %w[aspirin ibuprofen naproxen diclofenac],
      existing_terms: %w[warfarin],
      severity: 'high',
      description: 'May increase bleeding risk when used together. Review before adding or prescribing.'
    )
  ].freeze

  SEVERITY_LABELS = {
    'high' => 'High',
    'moderate' => 'Moderate',
    'low' => 'Low'
  }.freeze

  def initialize(medication_scope:)
    @medication_scope = medication_scope
  end

  def call(search_result)
    candidate_name = normalized_name(search_result.name.presence || search_result.display)
    return [] if candidate_name.blank?

    active_medications.filter_map do |medication|
      interaction_for(candidate_name, medication)
    end
  end

  private

  attr_reader :medication_scope

  def active_medications
    @active_medications ||= medication_scope.order(:name, :id).to_a
  end

  def interaction_for(candidate_name, medication)
    existing_name = normalized_name(medication.display_name)
    rule = RULES.find do |candidate_rule|
      term_match?(candidate_name, candidate_rule.candidate_terms) &&
        term_match?(existing_name, candidate_rule.existing_terms)
    end
    return unless rule

    {
      severity: rule.severity,
      severity_label: SEVERITY_LABELS.fetch(rule.severity, rule.severity.titleize),
      interacting_medication_name: medication.display_name,
      description: rule.description
    }
  end

  def term_match?(name, terms)
    terms.any? { |term| name.include?(term) }
  end

  def normalized_name(name)
    name.to_s.downcase.squish
  end
end
