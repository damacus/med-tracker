# frozen_string_literal: true

class MedicationReviewEvidenceCorpus
  Identity = Data.define(:terms, :classes)

  def initialize(records)
    @records = records
  end

  def matches_for(first_name, second_name)
    first_identity = identity_for(first_name)
    second_identity = identity_for(second_name)

    records.select do |record|
      record.match_pair?(candidate_name: first_name, existing_name: second_name) ||
        automatic_match?(record, first_name, first_identity, second_identity) ||
        automatic_match?(record, second_name, second_identity, first_identity)
    end
  end

  def owner?(record, medication_name)
    medication_term = MedicationReviewTermNormalizer.medication(medication_name)
    record.candidate_terms.any? { |term| overlapping_terms?(medication_term, normalize(term)) }
  end

  private

  attr_reader :records

  def identity_for(medication_name)
    medication_term = MedicationReviewTermNormalizer.medication(medication_name)
    identity_records = records.select { |record| owner?(record, medication_name) }
    Identity.new(
      terms: identity_terms(medication_term, identity_records),
      classes: identity_classes(identity_records)
    )
  end

  def identity_terms(medication_term, identity_records)
    ([medication_term] + normalized_record_terms(identity_records, :candidate_terms)).compact_blank.uniq
  end

  def identity_classes(identity_records)
    normalized_record_terms(identity_records, :pharmacologic_classes).compact_blank.uniq
  end

  def normalized_record_terms(identity_records, attribute)
    identity_records.flat_map(&attribute).map { |term| normalize(term) }
  end

  def automatic_match?(record, owner_name, owner_identity, other_identity)
    return false unless owner?(record, owner_name)
    return false if owner_identity.terms.empty? || other_identity.terms.empty?

    explicitly_mentions?(record.evidence_text, other_identity.terms + class_variants(other_identity.classes))
  end

  def explicitly_mentions?(text, terms)
    normalized_text = " #{normalize(text)} "
    terms.any? { |term| normalized_text.include?(" #{term} ") }
  end

  def class_variants(classes)
    classes.flat_map { |term| [term, pluralize_last_word(term)] }.uniq
  end

  def pluralize_last_word(term)
    words = term.split
    words[-1] = words.last.pluralize
    words.join(' ')
  end

  def overlapping_terms?(first, second)
    return false if first.blank? || second.blank?

    padded_first = " #{first} "
    padded_second = " #{second} "
    padded_first.include?(padded_second) || padded_second.include?(padded_first)
  end

  def normalize(value)
    MedicationReviewTermNormalizer.label(value)
  end
end
