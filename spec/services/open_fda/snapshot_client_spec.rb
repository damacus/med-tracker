# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::SnapshotClient do
  subject(:client) { described_class.new }

  it 'loads the complete reproducible public-label snapshot without network access' do
    labels = client.labels(limit: 80)

    expect(labels.size).to eq(80)
    expect(labels).to all(include('selection_term', 'set_id', 'id', 'effective_time', 'version', 'drug_interactions',
                                  'openfda'))
    expect(labels.map { |label| label.fetch('set_id') }).to all(be_present)
  end

  it 'rejects a request larger than the committed snapshot' do
    expect { client.labels(limit: 81) }.to raise_error(ArgumentError, /contains 80 labels/)
  end
end
