# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Medications::DoseOptionsPayloadPresenter do
  describe '#to_h' do
    let(:medication) do
      instance_double(Medication, id: 123, dose_options_payload: [{ 'amount' => '1' }])
    end

    it 'maps medications to their dose option payloads' do
      payload = described_class.new(medications: [medication]).to_h

      expect(payload).to eq('123' => [{ 'amount' => '1' }])
    end
  end
end
