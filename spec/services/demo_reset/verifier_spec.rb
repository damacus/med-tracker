# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::Verifier do
  subject(:verifier) do
    described_class.new(
      baseline_verifier:,
      auxiliary_verifier:,
      storage_empty:,
      demo_mode: -> { true }
    )
  end

  let(:baseline_verifier) { -> { { baseline: DemoBaseline::IDENTIFIER, accounts: 2 } } }
  let(:auxiliary_verifier) { -> { { queue: 0, cache: 0, cable: 0 } } }
  let(:storage_empty) { -> { true } }

  it 'reports only safe baseline, persistence, and storage invariants' do
    expect(verifier.call).to eq(
      baseline: DemoBaseline::IDENTIFIER,
      accounts: 2,
      auxiliary_databases: { queue: 0, cache: 0, cable: 0 },
      storage_empty: true
    )
  end

  it 'fails when the configured storage backend is not empty' do
    allow(storage_empty).to receive(:call).and_return(false)

    expect { verifier.call }.to raise_error(DemoReset::VerificationError, 'storage_not_empty')
  end
end
