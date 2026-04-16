# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::BarcodeLookup do
  describe '#lookup' do
    before do
      Rails.cache.clear
      NhsDmdBarcode.create!(
        gtin: '05016298210989',
        code: '13629411000001105',
        display: 'Laxido Orange oral powder sachets (Galen Ltd)',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    it 'finds the configured mapping for a string barcode' do
      result = described_class.new.lookup('5016298210989')

      expect(result).to include(
        code: '13629411000001105',
        display: 'Laxido Orange oral powder sachets (Galen Ltd)',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end
  end
end
