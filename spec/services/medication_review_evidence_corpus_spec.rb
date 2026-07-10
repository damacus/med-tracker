# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewEvidenceCorpus do
  it 'detects an unreviewed label that explicitly names the other ingredient' do
    evidence = evidence_record(
      source_record_id: 'explicit-ingredient',
      candidate_terms: ['warfarin sodium'],
      evidence_text: 'Concomitant use of warfarin with ibuprofen may increase bleeding risk.'
    )

    matches = described_class.new([evidence]).matches_for('Warfarin 1mg tablets', 'Ibuprofen 200mg tablets')

    expect(matches).to contain_exactly(evidence)
  end

  it 'detects an explicitly named class using public label class membership' do
    warfarin_evidence = evidence_record(
      source_record_id: 'explicit-class',
      candidate_terms: ['warfarin sodium'],
      evidence_text: 'Concomitant use with nonsteroidal anti-inflammatory drugs may increase bleeding risk.'
    )
    ibuprofen_identity = evidence_record(
      source_record_id: 'ibuprofen-identity',
      candidate_terms: ['ibuprofen'],
      pharmacologic_classes: ['nonsteroidal anti inflammatory drug'],
      evidence_text: 'No pairwise interaction statement in this section.'
    )

    matches = described_class.new([warfarin_evidence, ibuprofen_identity]).matches_for(
      'Warfarin 1mg tablets', 'Ibuprofen 200mg tablets'
    )

    expect(matches).to contain_exactly(warfarin_evidence)
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

  def evidence_record(source_record_id:, candidate_terms:, evidence_text:, pharmacologic_classes: [])
    MedicationReviewEvidenceRecord.new(
      source_name: 'DailyMed', source_record_id: source_record_id,
      source_url: "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=#{source_record_id}",
      retrieved_on: Date.new(2026, 7, 10), product_name: candidate_terms.first,
      active_ingredient: candidate_terms.first, label_section: 'Drug Interactions', evidence_text: evidence_text,
      risk_level: 'unknown', match_confidence: 'unknown', match_status: 'unreviewed',
      candidate_terms: candidate_terms, pharmacologic_classes: pharmacologic_classes, interacting_terms: []
    )
  end
end
