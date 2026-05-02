# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdBarcode do
  describe 'validations' do
    subject { build(:nhs_dmd_barcode) }

    it { is_expected.to validate_presence_of(:gtin) }
    it { is_expected.to validate_uniqueness_of(:gtin).case_insensitive }
    it { is_expected.to validate_presence_of(:code) }
    it { is_expected.to validate_presence_of(:display) }
    it { is_expected.to validate_presence_of(:system) }
  end

  describe '.normalize_gtin' do
    it 'removes non-digit characters' do
      expect(described_class.normalize_gtin('501-629-821-098-9')).to eq('5016298210989')
    end

    it 'handles strings that are already normalized' do
      expect(described_class.normalize_gtin('5016298210989')).to eq('5016298210989')
    end

    it 'handles nil input' do
      expect(described_class.normalize_gtin(nil)).to eq('')
    end

    it 'handles non-string input' do
      expect(described_class.normalize_gtin(5016298210989)).to eq('5016298210989')
    end
  end

  describe 'callbacks' do
    describe 'after_commit :expire_cache' do
      let(:barcode) { create(:nhs_dmd_barcode) }

      it 'calls NhsDmd::BarcodeLookup.expire with the gtin' do
        allow(NhsDmd::BarcodeLookup).to receive(:expire)
        barcode.update!(display: 'Updated Display')
        expect(NhsDmd::BarcodeLookup).to have_received(:expire).with(barcode.gtin)
      end
    end
  end
end
