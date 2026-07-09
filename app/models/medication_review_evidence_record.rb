# frozen_string_literal: true

class MedicationReviewEvidenceRecord < ApplicationRecord
  RISK_LEVELS = %w[low moderate high unknown].freeze
  MATCH_CONFIDENCES = %w[low moderate high unknown].freeze
  MATCH_STATUSES = %w[unreviewed reviewed_pair not_pairwise].freeze

  has_many :medication_review_prompts, foreign_key: :evidence_record_id, dependent: :restrict_with_error,
                                       inverse_of: :evidence_record

  validates :source_name, :source_record_id, :source_url, :retrieved_on, :product_name, :label_section,
            :evidence_text, presence: true
  validates :source_record_id, uniqueness: true
  validates :risk_level, inclusion: { in: RISK_LEVELS }
  validates :match_confidence, inclusion: { in: MATCH_CONFIDENCES }
  validates :match_status, inclusion: { in: MATCH_STATUSES }
  validate :reviewed_pair_has_terms

  scope :reviewable, -> { where(match_status: 'reviewed_pair') }

  def match_pair?(candidate_name:, existing_name:)
    return false unless match_status == 'reviewed_pair'

    direct_pair?(candidate_name, existing_name) || direct_pair?(existing_name, candidate_name)
  end

  private

  def direct_pair?(candidate_name, existing_name)
    term_match?(candidate_name, candidate_terms) && term_match?(existing_name, interacting_terms)
  end

  def term_match?(name, terms)
    normalized_name = name.to_s.downcase.squish
    terms.any? { |term| normalized_name.include?(term.to_s.downcase.squish) }
  end

  def reviewed_pair_has_terms
    return unless match_status == 'reviewed_pair'

    errors.add(:candidate_terms, 'must identify the first medicine') if candidate_terms.empty?
    errors.add(:interacting_terms, 'must identify the second medicine') if interacting_terms.empty?
  end
end
