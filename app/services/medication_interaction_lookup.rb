# frozen_string_literal: true

class MedicationInteractionLookup
  ReviewPrompt = Data.define(:candidate_terms, :existing_terms, :risk_level, :description)

  SOURCE_NAME = 'MedTracker review prompt seed data'

  RULES = [
    ReviewPrompt.new(
      candidate_terms: %w[warfarin],
      existing_terms: %w[aspirin ibuprofen naproxen diclofenac],
      risk_level: 'high',
      description: 'May increase bleeding risk when used together. Review with a pharmacist, nurse, GP, or prescriber.'
    ),
    ReviewPrompt.new(
      candidate_terms: %w[aspirin ibuprofen naproxen diclofenac],
      existing_terms: %w[warfarin],
      risk_level: 'high',
      description: 'May increase bleeding risk when used together. Review with a pharmacist, nurse, GP, or prescriber.'
    )
  ].freeze

  RISK_LEVEL_LABELS = {
    'high' => 'High',
    'moderate' => 'Moderate',
    'low' => 'Low',
    'unknown' => 'Unknown - unclassified'
  }.freeze

  def self.risk_level_label(risk_level)
    RISK_LEVEL_LABELS.fetch(risk_level, risk_level.to_s.titleize)
  end

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
      risk_level: rule.risk_level,
      risk_level_label: self.class.risk_level_label(rule.risk_level),
      interacting_medication_name: medication.display_name,
      source_name: SOURCE_NAME,
      source_checked_on: Date.current.iso8601,
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
