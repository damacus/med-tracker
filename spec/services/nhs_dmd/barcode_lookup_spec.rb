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

    it 'finds the configured mapping for a zero-prefixed barcode' do
      result = described_class.new.lookup('05016298210989')

      expect(result).to include(
        code: '13629411000001105',
        display: 'Laxido Orange oral powder sachets (Galen Ltd)',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    it 'includes optional trade family and group metadata for an active AMP mapping' do
      group = NhsDmdTradeFamilyGroup.create!(code: '900', name: 'Galen')
      family = NhsDmdTradeFamily.create!(code: '800', name: 'Laxido', trade_family_group: group)
      NhsDmdAmpTradeFamily.create!(amp_code: '222', trade_family: family)
      NhsDmdBarcode.find_by!(gtin: '05016298210989').update!(amp_code: '222')

      result = described_class.new.lookup('5016298210989')

      expect(result).to include(
        trade_family: { code: '800', name: 'Laxido' },
        trade_family_group: { code: '900', name: 'Galen' }
      )
    end

    it 'returns nil for a blank barcode' do
      expect(described_class.new.lookup('')).to be_nil
    end

    it 'returns nil when the barcode is not mapped' do
      expect(described_class.new.lookup('0000')).to be_nil
    end
  end
end
