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

  def initialize(text)
    @text = text.to_s.dup.freeze
  end

  def call(matched_term:)
    excerpt = excerpt_for(MedicationReviewTermNormalizer.label(matched_term))
    instruction, risk_level = classification(excerpt)
    Result.new(instruction: instruction, risk_level: risk_level, excerpt: excerpt)
  end

  private

  attr_reader :text

  def classification(excerpt)
    normalized_excerpt = MedicationReviewTermNormalizer.label(excerpt)
    return %w[no_action_required low] if NO_ACTION_PATTERNS.any? { |pattern| normalized_excerpt.match?(pattern) }

    match = CLASSIFICATIONS.find { |_instruction, _risk_level, pattern| normalized_excerpt.match?(pattern) }
    match ? match.first(2) : %w[unclassified unknown]
  end

  def excerpt_for(term)
    padded_term = " #{term} "
    matching_sentences = sentences.filter_map do |sentence, normalized|
      sentence if normalized.include?(padded_term)
    end
    matching_sentences.presence&.join(' ') || text
  end

  def sentences
    @sentences ||= text.split(/(?<=[.!?])\s+/).compact_blank.map do |sentence|
      [sentence, " #{MedicationReviewTermNormalizer.label(sentence)} "]
    end
  end
end
