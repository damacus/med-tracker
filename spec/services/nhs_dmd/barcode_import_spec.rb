# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'

RSpec.describe NhsDmd::BarcodeImport do
  describe '#import_csv' do
    let(:importer) { described_class.new }
    let(:path) { Rails.root.join('tmp/barcode-import-spec.csv') }

    after do
      FileUtils.rm_f(path)
    end

    it 'imports dm+d barcode mappings into the database' do
      File.write(path, <<~CSV)
        gtin,code,display,system,concept_class
        5016298210989,13629411000001105,Laxido Orange oral powder sachets (Galen Ltd),https://dmd.nhs.uk,AMPP
      CSV

      result = importer.import_csv(path)

      expect(result.imported_count).to eq(1)
      expect(result.invalid_rows).to eq([])
      expect(NhsDmdBarcode.find_by!(gtin: '5016298210989')).to have_attributes(
        code: '13629411000001105',
        display: 'Laxido Orange oral powder sachets (Galen Ltd)',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
    end

    it 'rejects files without the required headers' do
      File.write(path, <<~CSV)
        barcode,snomed,name
        5016298210989,13629411000001105,Laxido Orange oral powder sachets (Galen Ltd)
      CSV

      expect { importer.import_csv(path) }
        .to raise_error(ArgumentError, 'Missing required headers: gtin, code, display')
    end
  end
end
