# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::WormVerifierFactory do
  let(:adapter) { instance_double(Audit::ObjectLock::S3Adapter, validate!: true) }
  let(:configuration) { instance_double(Audit::ObjectLock::Configuration) }

  it 'validates Object Lock before building a verifier for the filtered records' do
    allow(Audit::ObjectLock::Configuration).to receive(:new).and_return(configuration)
    allow(Audit::ObjectLock::S3Adapter).to receive(:new).with(configuration:).and_return(adapter)

    verifier = described_class.new({}).call

    expect(adapter).to have_received(:validate!)
    expect(verifier).to be_a(Audit::Verification::WormVerifier)
  end

  it 'reports invalid filters as configuration failures' do
    expect do
      described_class.new('HOUSEHOLD_ID' => 'invalid').call
    end.to raise_error(Audit::Verification::ConfigurationError, 'invalid HOUSEHOLD_ID')
  end
end
