# frozen_string_literal: true

class MedicationReviewEvidenceCorpus
  Identity = Data.define(:terms, :classes)
  Match = Data.define(
    :evidence,
    :matched_term,
    :match_type,
    :match_confidence,
    :source_instruction,
    :risk_level,
    :evidence_excerpt,
    :reason
  )

  def initialize(records, terminology: MedicationReviewTerminology.new)
    @records = records
    @terminology = terminology
    @identities = {}
    @ownership = {}
    @records_by_medication = {}
  end

  def matches_for(first_name, second_name)
    first_identity = identity_for(first_name)
    second_identity = identity_for(second_name)

    pair_records(first_name, second_name).filter_map do |record|
      curated_match(record, first_name, second_name) ||
        automatic_match(record, first_name, first_identity, second_identity) ||
        automatic_match(record, second_name, second_identity, first_identity)
    end
  end

  def owner?(record, medication_name)
    medication_term = MedicationReviewTermNormalizer.medication(medication_name)
    key = [record.object_id, medication_term]

    ownership.fetch(key) do
      ownership[key] = record.candidate_terms.any? { |term| overlapping_terms?(medication_term, normalize(term)) }
    end
  end

  private

  attr_reader :records, :terminology, :identities, :ownership, :records_by_medication

  def identity_for(medication_name)
    identities.fetch(medication_name) do
      medication_term = MedicationReviewTermNormalizer.medication(medication_name)
      identity_records = records.select { |record| owner?(record, medication_name) }
      records_by_medication[medication_name] = identity_records
      terminology_identity = terminology.identity_for(medication_name)
      identities[medication_name] = Identity.new(
        terms: identity_terms(medication_term, identity_records, terminology_identity),
        classes: identity_classes(identity_records, terminology_identity)
      )
    end
  end

  def pair_records(first_name, second_name)
    (records_by_medication.fetch(first_name) + records_by_medication.fetch(second_name)).uniq
  end

  def identity_terms(medication_term, identity_records, terminology_identity)
    ([medication_term] + normalized_record_terms(identity_records,
                                                 :candidate_terms) + terminology_identity.fetch(:terms))
      .compact_blank.uniq
  end

  def identity_classes(identity_records, terminology_identity)
    (normalized_record_terms(identity_records, :pharmacologic_classes) + terminology_identity.fetch(:classes))
      .compact_blank.uniq
  end

  def normalized_record_terms(identity_records, attribute)
    identity_records.flat_map(&attribute).map { |term| normalize(term) }
  end

  def curated_match(record, first_name, second_name)
    return unless record.match_pair?(candidate_name: first_name, existing_name: second_name)

    matched_term = curated_matched_term(record, first_name, second_name)
    Match.new(
      evidence: record,
      matched_term: matched_term,
      match_type: 'curated',
      match_confidence: record.match_confidence,
      source_instruction: 'unclassified',
      risk_level: record.risk_level,
      evidence_excerpt: record.evidence_text,
      reason: "A reviewed rule identifies #{matched_term} as the interacting medicine."
    )
  end

  def curated_matched_term(record, first_name, second_name)
    record.interacting_terms.find do |term|
      [first_name, second_name].any? { |name| overlapping_terms?(normalize(name), normalize(term)) }
    end || record.interacting_terms.first
  end

  def automatic_match(record, owner_name, owner_identity, other_identity)
    return unless owner?(record, owner_name)
    return if owner_identity.terms.empty? || other_identity.terms.empty?

    matched_term = explicit_term(record.evidence_text, other_identity.terms)
    return build_automatic_match(record, matched_term, 'ingredient', 'high') if matched_term

    matched_term = explicit_term(record.evidence_text, class_variants(other_identity.classes))
    return unless matched_term

    build_automatic_match(record, matched_term, 'class', 'moderate')
  end

  def build_automatic_match(record, matched_term, match_type, confidence)
    classification = MedicationReviewSourceInstructionClassifier.new(
      record.evidence_text, matched_term: matched_term
    ).call
    return if classification.instruction == 'no_action_required'

    Match.new(
      evidence: record,
      matched_term: matched_term,
      match_type: match_type,
      match_confidence: confidence,
      source_instruction: classification.instruction,
      risk_level: classification.risk_level,
      evidence_excerpt: classification.excerpt,
      reason: "The label explicitly names the #{match_type} #{matched_term}."
    )
  end

  def explicit_term(text, terms)
    normalized_text = " #{normalize(text)} "
    terms.sort_by { |term| -term.length }.find { |term| normalized_text.include?(" #{term} ") }
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
