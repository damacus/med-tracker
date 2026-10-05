# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::Tools::ExtractMedicationGuidance do
  subject(:tool) { described_class.new }

  let(:valid_suggestion_payload) do
    {
      'medication' => { 'description' => 'Paracetamol pain relief' },
      'doses' => [
        {
          'amount' => 5,
          'unit' => 'ml',
          'description' => 'Children 6-8 years',
          'default_max_daily_doses' => 4,
          'default_min_hours_between_doses' => 4,
          'default_dose_cycle' => 'daily',
          'evidence' => {
            'url' => 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol',
            'title' => 'CALPOL SixPlus',
            'text' => 'Children 6-8 years 5ml Up to 4 times in 24 hours'
          }
        }
      ],
      'sources' => [
        { 'url' => 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol',
          'title' => 'CALPOL SixPlus' }
      ]
    }
  end

  describe '#execute' do
    it 'returns the validated suggestion as JSON' do
      result = tool.execute(suggestion: valid_suggestion_payload)

      expect(result).to include(medication: include('description' => 'Paracetamol pain relief'))
    end

    it 'passes suggestion through SuggestionValidator' do
      validator = instance_double(AiMedication::SuggestionValidator)
      validated_suggestion = AiMedication::Suggestion.new(
        medication: { 'description' => 'Paracetamol pain relief' },
        doses: valid_suggestion_payload['doses'],
        sources: valid_suggestion_payload['sources']
      )
      allow(AiMedication::SuggestionValidator).to receive(:new).and_return(validator)
      allow(validator).to receive(:call).and_return(validated_suggestion)

      tool.execute(suggestion: valid_suggestion_payload)

      expect(validator).to have_received(:call).with(an_instance_of(AiMedication::Suggestion))
    end

    it 'handles missing medication key gracefully' do
      payload = valid_suggestion_payload.except('medication')
      result = tool.execute(suggestion: payload)

      expect(result).to include(medication: {})
    end

    it 'handles missing doses key gracefully' do
      payload = valid_suggestion_payload.except('doses')
      result = tool.execute(suggestion: payload)

      expect(result).to include(doses: [])
    end

    it 'handles missing sources key gracefully' do
      payload = valid_suggestion_payload.except('sources')
      result = tool.execute(suggestion: payload)

      expect(result).to include(sources: [])
    end

    it 'returns an error hash when an unexpected exception is raised' do
      allow(AiMedication::SuggestionValidator).to receive(:new).and_raise(StandardError, 'unexpected failure')

      result = tool.execute(suggestion: valid_suggestion_payload)

      expect(result).to include(error: 'invalid_suggestion', message: 'unexpected failure')
    end
  end
end
