# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewSourceInstructionClassifier do
  it 'classifies contraindicated source wording as high' do
    result = described_class.new('Use with phenelzine is contraindicated.').call(matched_term: 'phenelzine')

    expect(result).to have_attributes(instruction: 'contraindicated', risk_level: 'high')
  end

  it 'classifies explicit avoidance source wording as high' do
    result = described_class.new('Avoid concomitant use with clarithromycin.').call(matched_term: 'clarithromycin')

    expect(result).to have_attributes(instruction: 'avoid', risk_level: 'high')
  end

  it 'classifies monitoring or adjustment source wording as moderate' do
    result = described_class.new('Monitor INR and adjust the warfarin dose.').call(matched_term: 'warfarin')

    expect(result).to have_attributes(instruction: 'monitor_or_adjust', risk_level: 'moderate')
  end

  it 'classifies possible or cautionary source wording as low' do
    result = described_class.new('Ibuprofen may increase bleeding risk.').call(matched_term: 'ibuprofen')

    expect(result).to have_attributes(instruction: 'possible_or_caution', risk_level: 'low')
  end

  it 'keeps unsupported mentions unclassified' do
    result = described_class.new('Phenytoin was included in the interaction study.').call(matched_term: 'phenytoin')

    expect(result).to have_attributes(instruction: 'unclassified', risk_level: 'unknown')
  end

  it 'recognises explicit no-adjustment wording as no action required' do
    text = 'No dosing adjustments required for ibuprofen.'

    result = described_class.new(text).call(matched_term: 'ibuprofen')

    expect(result).to have_attributes(instruction: 'no_action_required', risk_level: 'low')
  end

  it 'reuses prepared sentences while keeping instructions specific to each medicine' do
    text = 'Use with phenelzine is contraindicated. Monitor INR when warfarin is used.'
    allow(MedicationReviewTermNormalizer).to receive(:label).and_call_original
    classifier = described_class.new(text)

    expect(classifier.call(matched_term: 'phenelzine').instruction).to eq('contraindicated')
    expect(classifier.call(matched_term: 'warfarin').instruction).to eq('monitor_or_adjust')
    expect(MedicationReviewTermNormalizer).to have_received(:label)
      .with('Use with phenelzine is contraindicated.').twice
  end

  it 'falls back to the whole label when no sentence names the medicine' do
    text = 'Monitor patient response. Adjust the dose as needed.'

    result = described_class.new(text).call(matched_term: 'warfarin')

    expect(result).to have_attributes(instruction: 'monitor_or_adjust', excerpt: text)
  end

  it 'classifies only the sentence containing the matched medicine' do
    text = 'Use with phenelzine is contraindicated. Monitor INR when warfarin is used.'

    result = described_class.new(text).call(matched_term: 'warfarin')

    expect(result).to have_attributes(instruction: 'monitor_or_adjust', risk_level: 'moderate')
    expect(result.excerpt).to eq('Monitor INR when warfarin is used.')
  end
end
