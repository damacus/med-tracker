# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'

RSpec.describe BarcodeCatalog::ImportCsv do
  describe '#call' do
    let(:importer) { described_class.new }
    let(:path) { Rails.root.join('tmp/barcode-catalog-import-spec.csv') }

    after do
      FileUtils.rm_f(path)
    end

    it 'imports external barcode catalogue mappings' do
      File.write(path, <<~CSV)
        gtin,display,source,code,system,concept_class
        3574661385488,Calprofen 100mg/5ml oral suspension,cd_data,4585411000001109,https://dmd.nhs.uk,AMPP
      CSV

      result = importer.call(path)

      expect(result.imported_count).to eq(1)
      expect(result.invalid_rows).to eq([])
      expect(BarcodeCatalogEntry.find_by!(gtin: '3574661385488')).to have_attributes(
        display: 'Calprofen 100mg/5ml oral suspension',
        source: 'cd_data',
        code: '4585411000001109',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    it 'rejects files without the required headers' do
      File.write(path, <<~CSV)
        barcode,name
        3574661385488,Calprofen 100mg/5ml oral suspension
      CSV

      expect { importer.call(path) }
        .to raise_error(ArgumentError, 'Missing required headers: gtin, display, source')
    end
  end
end
