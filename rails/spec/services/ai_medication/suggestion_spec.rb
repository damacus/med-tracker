# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::Suggestion do
  describe '#initialize' do
    it 'stringifies medication keys' do
      suggestion = described_class.new(medication: { name: 'Calpol', strength: '120mg' })

      expect(suggestion.medication).to eq('name' => 'Calpol', 'strength' => '120mg')
    end

    it 'stringifies keys in each dose hash' do
      suggestion = described_class.new(doses: [{ amount: 5, unit: 'ml' }])

      expect(suggestion.doses).to eq([{ 'amount' => 5, 'unit' => 'ml' }])
    end

    it 'stringifies keys in each source hash' do
      suggestion = described_class.new(sources: [{ url: 'https://example.com', title: 'Example' }])

      expect(suggestion.sources).to eq([{ 'url' => 'https://example.com', 'title' => 'Example' }])
    end

    it 'keeps errors as given' do
      suggestion = described_class.new(errors: %w[ruby_llm_unavailable])

      expect(suggestion.errors).to eq(%w[ruby_llm_unavailable])
    end

    it 'defaults all attributes to empty' do
      suggestion = described_class.new

      expect(suggestion.medication).to eq({})
      expect(suggestion.doses).to eq([])
      expect(suggestion.sources).to eq([])
      expect(suggestion.errors).to eq([])
    end
  end

  describe '#empty?' do
    it 'is true when medication and doses are both blank' do
      expect(described_class.new).to be_empty
    end

    it 'is false when medication is present' do
      suggestion = described_class.new(medication: { name: 'Calpol' })

      expect(suggestion).not_to be_empty
    end

    it 'is false when doses are present' do
      suggestion = described_class.new(doses: [{ amount: 5, unit: 'ml' }])

      expect(suggestion).not_to be_empty
    end

    it 'is true when only errors are present' do
      suggestion = described_class.new(errors: ['ruby_llm_unavailable'])

      expect(suggestion).to be_empty
    end
  end

  describe '#as_json' do
    it 'returns all four keys' do
      suggestion = described_class.new(
        medication: { name: 'Calpol' },
        doses: [{ amount: 5, unit: 'ml' }],
        sources: [{ url: 'https://example.com', title: 'Example' }],
        errors: []
      )
      json = suggestion.as_json

      expect(json).to include(
        medication: { 'name' => 'Calpol' },
        doses: [{ 'amount' => 5, 'unit' => 'ml' }],
        sources: [{ 'url' => 'https://example.com', 'title' => 'Example' }],
        errors: []
      )
    end

    it 'includes errors when present' do
      suggestion = described_class.new(errors: ['ruby_llm_unavailable'])

      expect(suggestion.as_json[:errors]).to eq(['ruby_llm_unavailable'])
    end
  end
end
