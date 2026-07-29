require 'rails_helper'

RSpec.describe MedicationTradeFamilyResolver do
  subject(:resolver) { described_class.new(scope: scope) }

  let(:exact_medication) { create(:medication, barcode: '5016298210989') }
  let(:related_medication) { create(:medication, barcode: '5016298210996') }
  let(:unrelated_medication) { create(:medication, barcode: '5016298211009') }
  let(:scope) { Medication.where(id: [exact_medication.id, related_medication.id, unrelated_medication.id]) }

  before do
    matching_family = NhsDmdTradeFamily.create!(code: 'TF001', name: 'Laxido')
    unrelated_family = NhsDmdTradeFamily.create!(code: 'TF002', name: 'Other medicine')
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP001', trade_family: matching_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP002', trade_family: matching_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP003', trade_family: unrelated_family)
    NhsDmdBarcode.create!(
      gtin: exact_medication.barcode, code: 'AMPP001', amp_code: 'AMP001', display: exact_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: related_medication.barcode, code: 'AMPP002', amp_code: 'AMP002', display: related_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: unrelated_medication.barcode, code: 'AMPP003', amp_code: 'AMP003', display: unrelated_medication.name
    )
  end

  describe '#call' do
    it 'finds household medicines whose barcode resolves to the requested trade family' do
      related_medications = resolver.call(trade_family_code: 'TF001', excluding: exact_medication)

      expect(related_medications).to contain_exactly(related_medication)
    end

    it 'does not return medicines in another trade family' do
      expect(resolver.call(trade_family_code: 'TF002')).to contain_exactly(unrelated_medication)
    end

    it 'returns no medicines when the finder result has no trade family' do
      expect(resolver.call(trade_family_code: nil)).to be_empty
    end
  end
end
