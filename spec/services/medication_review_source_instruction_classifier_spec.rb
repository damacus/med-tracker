# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewSourceInstructionClassifier do
  it 'classifies contraindicated source wording as high' do
    result = described_class.new('Use with phenelzine is contraindicated.', matched_term: 'phenelzine').call

    expect(result).to have_attributes(instruction: 'contraindicated', risk_level: 'high')
  end

  it 'classifies explicit avoidance source wording as high' do
    result = described_class.new('Avoid concomitant use with clarithromycin.', matched_term: 'clarithromycin').call

    expect(result).to have_attributes(instruction: 'avoid', risk_level: 'high')
  end

  it 'classifies monitoring or adjustment source wording as moderate' do
    result = described_class.new('Monitor INR and adjust the warfarin dose.', matched_term: 'warfarin').call

    expect(result).to have_attributes(instruction: 'monitor_or_adjust', risk_level: 'moderate')
  end

  it 'classifies possible or cautionary source wording as low' do
    result = described_class.new('Ibuprofen may increase bleeding risk.', matched_term: 'ibuprofen').call

    expect(result).to have_attributes(instruction: 'possible_or_caution', risk_level: 'low')
  end

  it 'keeps unsupported mentions unclassified' do
    result = described_class.new('Phenytoin was included in the interaction study.', matched_term: 'phenytoin').call

    expect(result).to have_attributes(instruction: 'unclassified', risk_level: 'unknown')
  end

  it 'classifies only the sentence containing the matched medicine' do
    text = 'Use with phenelzine is contraindicated. Monitor INR when warfarin is used.'

    result = described_class.new(text, matched_term: 'warfarin').call

    expect(result).to have_attributes(instruction: 'monitor_or_adjust', risk_level: 'moderate')
    expect(result.excerpt).to eq('Monitor INR when warfarin is used.')
  end
end
