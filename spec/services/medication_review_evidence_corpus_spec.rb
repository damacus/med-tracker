# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewEvidenceCorpus do
  it 'detects an unreviewed label that explicitly names the other ingredient' do
    evidence = explicit_ingredient_evidence
    match = described_class.new([evidence]).matches_for('Warfarin 1mg tablets', 'Ibuprofen 200mg tablets').sole

    expect(match).to have_attributes(
      evidence: evidence,
      matched_term: 'ibuprofen',
      match_type: 'ingredient',
      match_confidence: 'high',
      source_instruction: 'possible_or_caution',
      risk_level: 'low'
    )
    expect(match.evidence_excerpt).to eq(evidence.evidence_text)
  end

  it 'detects an explicitly named class using public label class membership' do
    warfarin_evidence, ibuprofen_identity = explicit_class_records
    match = described_class.new([warfarin_evidence, ibuprofen_identity]).matches_for(
      'Warfarin 1mg tablets', 'Ibuprofen 200mg tablets'
    ).sole

    expect(match).to have_attributes(
      evidence: warfarin_evidence,
      matched_term: 'nonsteroidal anti inflammatory drugs',
      match_type: 'class',
      match_confidence: 'moderate'
    )
  end

  it 'does not infer an interaction from mechanism-only text' do
    evidence = evidence_record(
      source_record_id: 'mechanism-only',
      candidate_terms: ['warfarin sodium'],
      evidence_text: 'Warfarin is metabolized through CYP2C9 pathways.'
    )

    matches = described_class.new([evidence]).matches_for('Warfarin', 'Ibuprofen')

    expect(matches).to be_empty
  end

  it 'preserves manually reviewed pair confidence while exposing a match reason' do
    evidence = evidence_record(
      source_record_id: 'reviewed-pair',
      candidate_terms: ['warfarin'],
      evidence_text: 'Reviewed public-label evidence.',
      match_status: 'reviewed_pair',
      match_confidence: 'high',
      interacting_terms: ['ibuprofen']
    )

    match = described_class.new([evidence]).matches_for('Warfarin', 'Ibuprofen').sole

    expect(match).to have_attributes(
      evidence: evidence,
      matched_term: 'ibuprofen',
      match_type: 'curated',
      match_confidence: 'high'
    )
  end

  it 'uses public terminology classes even when the medicine has no local label identity record' do
    sildenafil_evidence = evidence_record(
      source_record_id: 'sildenafil-nitrates',
      candidate_terms: ['sildenafil'],
      evidence_text: 'Use with nitrates is contraindicated.'
    )
    match = described_class.new([sildenafil_evidence], terminology: nitrate_terminology)
                           .matches_for('Sildenafil', 'Nitroglycerin').sole

    expect(match).to have_attributes(
      matched_term: 'nitrates', match_type: 'class', source_instruction: 'contraindicated', risk_level: 'high'
    )
  end

  def evidence_record(**overrides)
    defaults = {
      source_record_id: 'evidence-record', candidate_terms: ['medicine'], evidence_text: 'Public label evidence.',
      pharmacologic_classes: [], match_status: 'unreviewed', match_confidence: 'unknown', interacting_terms: []
    }
    attributes = defaults.merge(overrides)
    MedicationReviewEvidenceRecord.new(
      source_name: 'DailyMed', source_record_id: attributes.fetch(:source_record_id),
      source_version: '4', source_effective_on: Date.new(2026, 7, 1),
      source_url: "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=#{attributes.fetch(:source_record_id)}",
      retrieved_on: Date.new(2026, 7, 10), product_name: attributes.fetch(:candidate_terms).first,
      active_ingredient: attributes.fetch(:candidate_terms).first, label_section: 'Drug Interactions',
      risk_level: 'unknown', **attributes
    )
  end

  def explicit_ingredient_evidence
    evidence_record(
      source_record_id: 'explicit-ingredient', candidate_terms: ['warfarin sodium'],
      evidence_text: 'Concomitant use of warfarin with ibuprofen may increase bleeding risk.'
    )
  end

  def explicit_class_records
    [
      evidence_record(
        source_record_id: 'explicit-class', candidate_terms: ['warfarin sodium'],
        evidence_text: 'Concomitant use with nonsteroidal anti-inflammatory drugs may increase bleeding risk.'
      ),
      evidence_record(source_record_id: 'ibuprofen-identity', candidate_terms: ['ibuprofen'],
                      pharmacologic_classes: ['nonsteroidal anti inflammatory drug'])
    ]
  end

  def nitrate_terminology
    MedicationReviewTerminology.new(entries: [nitrate_entry], aliases: [])
  end

  def nitrate_entry
    {
      'selection_term' => 'nitroglycerin', 'rxcui' => '4917', 'ingredient_name' => 'nitroglycerin',
      'classes' => [{ 'id' => 'D009566', 'name' => 'Nitrates', 'type' => 'CHEM' }]
    }
  end
end
