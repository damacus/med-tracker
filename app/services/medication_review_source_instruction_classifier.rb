# frozen_string_literal: true

class MedicationReviewSourceInstructionClassifier
  Result = Data.define(:instruction, :risk_level, :excerpt)
  CLASSIFICATIONS = [
    ['contraindicated', 'high', /\bcontraindicat/],
    ['avoid', 'high', /\bavoid\b|\bshould not be used\b|\bnot recommended\b|\bdo not use\b/],
    ['monitor_or_adjust', 'moderate', /\bmonitor|\badjust|\bdose reduction|\breduce(?:d)? dose|\bclosely observe/],
    ['possible_or_caution', 'low', /\bmay\b|\bcan\b|\bcaution|\brisk\b|\bpotential/]
  ].freeze
  NO_ACTION_PATTERNS = [
    /\bno (?:dose|dosing) adjustments? (?:(?:is|are) )?(?:needed|required|necessary)\b/,
    /\bno clinically (?:meaningful|relevant|significant) (?:change|effect|interaction)\b/
  ].freeze

  def initialize(text, matched_term:)
    @text = text.to_s
    @matched_term = MedicationReviewTermNormalizer.label(matched_term)
  end

  def call
    instruction, risk_level = classification
    Result.new(instruction: instruction, risk_level: risk_level, excerpt: excerpt)
  end

  private

  attr_reader :text, :matched_term

  def classification
    normalized_excerpt = MedicationReviewTermNormalizer.label(excerpt)
    return %w[no_action_required low] if NO_ACTION_PATTERNS.any? { |pattern| normalized_excerpt.match?(pattern) }

    match = CLASSIFICATIONS.find { |_instruction, _risk_level, pattern| normalized_excerpt.match?(pattern) }
    match ? match.first(2) : %w[unclassified unknown]
  end

  def excerpt
    @excerpt ||= begin
      matching_sentences = sentences.select { |sentence| contains_term?(sentence, matched_term) }
      matching_sentences.presence&.join(' ') || text
    end
  end

  def sentences
    text.split(/(?<=[.!?])\s+/).compact_blank
  end

  def contains_term?(value, term)
    " #{MedicationReviewTermNormalizer.label(value)} ".include?(" #{term} ")
  end
end
