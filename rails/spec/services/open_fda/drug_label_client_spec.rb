# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::DrugLabelClient do
  it 'prefers a matching human monotherapy label over a combination product' do
    response = {
      'meta' => { 'last_updated' => '2026-07-10' },
      'results' => [
        label('combination', ['ACETAMINOPHEN', 'OXYCODONE HYDROCHLORIDE']),
        label('monotherapy', ['OXYCODONE HYDROCHLORIDE'])
      ]
    }
    result = client_with(response).labels_for(['oxycodone'])

    expect(result.fetch('results').sole.fetch('set_id')).to eq('monotherapy')
  end

  it 'requires every configured target in targeted interaction labels' do
    response = {
      'meta' => { 'last_updated' => '2026-07-10' },
      'results' => [label('doxazosin-label', ['DOXAZOSIN MESYLATE'])]
    }
    client = client_with(response)
    targeted_selection = [
      { 'term' => 'doxazosin', 'interaction_targets' => %w[ibuprofen acetaminophen] }
    ]

    result = client.labels_for_targeted(targeted_selection)

    expect(result.fetch('results').sole.fetch('set_id')).to eq('doxazosin-label')
    expect(client.requested_entries).to eq(targeted_selection)
  end

  def client_with(response)
    client_class = Class.new(described_class) do
      attr_reader :requested_entries

      define_method(:concurrent_responses) do |entries|
        @requested_entries = entries
        [response]
      end
    end
    client_class.new
  end

  def label(set_id, substances)
    {
      'set_id' => set_id,
      'openfda' => {
        'product_type' => ['HUMAN PRESCRIPTION DRUG'],
        'substance_name' => substances
      }
    }
  end
end
