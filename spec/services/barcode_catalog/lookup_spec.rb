# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BarcodeCatalog::Lookup do
  describe '#lookup' do
    let(:lookup) { described_class.new }
    let(:gtin) { '3574661385488' }

    def create_external_entry
      BarcodeCatalogEntry.create!(
        gtin: gtin,
        display: 'Calprofen 100mg/5ml oral suspension',
        source: 'cd_data',
        code: '4585411000001109',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    def create_local_entry(code:, display:)
      NhsDmdBarcode.create!(
        gtin: gtin,
        code: code,
        display: display,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    it 'prefers an external catalogue entry over the local dm+d barcode table' do
      create_external_entry
      create_local_entry(code: 'old-code', display: 'Old dm+d mapping')

      expect(lookup.lookup(gtin)).to include(
        display: 'Calprofen 100mg/5ml oral suspension',
        code: '4585411000001109',
        source: 'cd_data'
      )
    end

    it 'falls back to the local dm+d barcode table when no external catalogue entry exists' do
      create_local_entry(
        code: '4585411000001109',
        display: 'Calprofen 100mg/5ml oral suspension (McNeil Products Ltd) 100 ml'
      )

      expect(lookup.lookup(gtin)).to include(
        display: 'Calprofen 100mg/5ml oral suspension (McNeil Products Ltd) 100 ml',
        code: '4585411000001109',
        source: 'nhs_dmd'
      )
    end
  end
end
