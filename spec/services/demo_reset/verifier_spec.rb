# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::Verifier do
  subject(:verifier) do
    described_class.new(
      baseline_verifier:,
      auxiliary_verifier:,
      storage_root:,
      health_checker:,
      demo_mode: -> { true }
    )
  end

  let(:baseline_verifier) { -> { { baseline: DemoBaseline::IDENTIFIER, accounts: 2 } } }
  let(:auxiliary_verifier) { -> { { queue: 0, cache: 0, cable: 0 } } }
  let(:storage_root) { Pathname(Dir.mktmpdir('verified-demo-storage')) }
  let(:health_checker) { -> { true } }

  after { FileUtils.rm_rf(storage_root) }

  it 'reports only safe baseline, persistence, storage, and health invariants' do
    expect(verifier.call).to eq(
      baseline: DemoBaseline::IDENTIFIER,
      accounts: 2,
      auxiliary_databases: { queue: 0, cache: 0, cable: 0 },
      storage_entries: 0,
      health: 'available'
    )
  end

  it 'fails when the upload root is not empty' do
    storage_root.join('unexpected.bin').write('synthetic upload')

    expect { verifier.call }.to raise_error(DemoReset::VerificationError, 'storage_not_empty')
  end

  it 'fails when canary health is unavailable' do
    allow(health_checker).to receive(:call).and_return(false)

    expect { verifier.call }.to raise_error(DemoReset::VerificationError, 'application_unavailable')
  end
end
