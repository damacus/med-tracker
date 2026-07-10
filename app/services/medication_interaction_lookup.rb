# frozen_string_literal: true

class MedicationInteractionLookup
  Result = Data.define(:visible_prompts, :hidden_count)

  RISK_LEVEL_LABELS = {
    'high' => 'High',
    'moderate' => 'Moderate',
    'low' => 'Low',
    'unknown' => 'Unknown - unclassified'
  }.freeze
  MATCH_CONFIDENCE_LABELS = {
    'high' => 'High',
    'moderate' => 'Moderate',
    'low' => 'Low',
    'unknown' => 'Unknown - unclassified'
  }.freeze

  def self.risk_level_label(risk_level)
    RISK_LEVEL_LABELS.fetch(risk_level, risk_level.to_s.titleize)
  end

  def self.match_confidence_label(match_confidence)
    MATCH_CONFIDENCE_LABELS.fetch(match_confidence, match_confidence.to_s.titleize)
  end

  def initialize(medication_scope:, evidence_scope: MedicationReviewEvidenceRecord.detectable)
    @medication_scope = medication_scope
    @evidence_scope = evidence_scope
  end

  def call(search_result)
    candidate_name = normalized_name(search_result.name.presence || search_result.display)
    return empty_result if candidate_name.blank?

    visible_prompts, hidden_prompts = prompts_for(candidate_name).partition { |prompt| !hidden_low_signal?(prompt) }
    Result.new(visible_prompts: visible_prompts, hidden_count: hidden_prompts.size)
  end

  private

  attr_reader :medication_scope, :evidence_scope

  def empty_result
    Result.new(visible_prompts: [], hidden_count: 0)
  end

  def prompts_for(candidate_name)
    active_medications.flat_map do |medication|
      evidence_corpus.matches_for(candidate_name, medication.display_name).map do |evidence|
        prompt_metadata(medication, evidence).merge(evidence_metadata(evidence))
      end
    end
  end

  def prompt_metadata(medication, evidence)
    {
      evidence_record_id: evidence.id,
      risk_level: evidence.risk_level,
      risk_level_label: self.class.risk_level_label(evidence.risk_level),
      match_confidence: evidence.match_confidence,
      match_confidence_label: self.class.match_confidence_label(evidence.match_confidence),
      interacting_medication_name: medication.display_name,
      description: review_description
    }
  end

  def evidence_metadata(evidence)
    {
      source_name: evidence.source_name,
      source_checked_on: evidence.retrieved_on.iso8601,
      source_url: evidence.source_url,
      evidence_text: evidence.evidence_text.truncate(500)
    }
  end

  def hidden_low_signal?(prompt)
    prompt[:risk_level] == 'low' || prompt[:match_confidence] == 'low'
  end

  def active_medications
    @active_medications ||= medication_scope.order(:name, :id).to_a
  end

  def evidence_records
    @evidence_records ||= evidence_scope.order(:id).to_a
  end

  def evidence_corpus
    @evidence_corpus ||= MedicationReviewEvidenceCorpus.new(evidence_records)
  end

  def normalized_name(name)
    name.to_s.downcase.squish
  end

  def review_description
    'Public medicine-label evidence suggests this combination may be worth reviewing with a pharmacist, nurse, ' \
      'GP, or prescriber.'
  end
end
