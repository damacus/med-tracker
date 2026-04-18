# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'

RSpec.describe NhsDmd::ReleaseImport do
  let(:importer) { described_class.new }
  let(:release_dir) { Rails.root.join('tmp/dmd-release-spec') }

  before { FileUtils.mkdir_p(release_dir) }
  after { FileUtils.rm_rf(release_dir) }

  def write_ampp_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS>'
    entries.each do |e|
      xml << "<AMPP><APPID>#{e[:appid]}</APPID><NM>#{e[:nm]}</NM></AMPP>"
    end
    xml << '</AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>'
    File.write(release_dir.join('f_ampp2_3000000.xml'), xml)
  end

  def write_gtin_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<GTIN_DETAILS><AMPPS>'
    xml << entries.map { |entry| gtin_ampp_xml(entry) }.join
    xml << '</AMPPS></GTIN_DETAILS>'
    File.write(release_dir.join('f_gtin2_0000000.xml'), xml)
  end

  def gtin_ampp_xml(entry)
    gtins = Array(entry[:gtins]).map { |gtin| gtin_data_xml(gtin) }.join
    "<AMPP><AMPPID>#{entry[:amppid]}</AMPPID>#{gtins}</AMPP>"
  end

  def gtin_data_xml(gtin)
    xml = "<GTINDATA><GTIN>#{gtin[:gtin]}</GTIN><STARTDT>#{gtin[:startdt]}</STARTDT>"
    xml << "<ENDDT>#{gtin[:enddt]}</ENDDT>" if gtin[:enddt]
    xml << '</GTINDATA>'
  end

  def write_single_gtin_xml(amppid:, gtin:, startdt:, enddt: nil)
    write_gtin_xml([{ amppid: amppid, gtins: [{ gtin: gtin, startdt: startdt, enddt: enddt }.compact] }])
  end

  def gtin_entry(amppid:, gtins:)
    { amppid: amppid, gtins: gtins }
  end

  def barcode_record(gtin)
    NhsDmdBarcode.find_by!(gtin: gtin)
  end

  it 'imports active GTINs matched to AMPP names' do
    write_ampp_xml([{ appid: '111', nm: 'Paracetamol 500mg tablets (Acme Ltd) 16 tablet' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(result.skipped_count).to eq(0)
    expect(barcode_record('5016298210989')).to have_attributes(
      code: '111',
      display: 'Paracetamol 500mg tablets (Acme Ltd) 16 tablet',
      system: 'https://dmd.nhs.uk',
      concept_class: 'AMPP'
    )
  end

  it 'skips GTINs with a past end date' do
    write_ampp_xml([{ appid: '222', nm: 'Expired Product 10mg tablets' }])
    write_single_gtin_xml(
      amppid: '222',
      gtin: '1234567890123',
      startdt: '2019-01-01',
      enddt: '2020-01-01'
    )

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(0)
    expect(NhsDmdBarcode.find_by(gtin: '1234567890123')).to be_nil
  end

  it 'keeps GTINs with a future end date' do
    write_ampp_xml([{ appid: '333', nm: 'Future Discontinue 25mg capsules' }])
    write_single_gtin_xml(
      amppid: '333',
      gtin: '9876543210987',
      startdt: '2020-01-01',
      enddt: '2099-12-31'
    )

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(barcode_record('9876543210987')).to have_attributes(code: '333')
  end

  it 'skips GTINs with no matching AMPP name' do
    write_ampp_xml([]) # no AMPPs
    write_single_gtin_xml(amppid: '444', gtin: '1111111111111', startdt: '2020-01-01')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(0)
    expect(result.skipped_count).to eq(1)
  end

  it 'imports multiple GTINs for the same AMPP' do
    write_ampp_xml([{ appid: '555', nm: 'Multi-barcode Product' }])
    gtins = [
      { gtin: '2222222222222', startdt: '2020-01-01' },
      { gtin: '3333333333333', startdt: '2021-01-01' }
    ]
    write_gtin_xml([gtin_entry(amppid: '555', gtins: gtins)])

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(2)
    expect(NhsDmdBarcode.where(code: '555').count).to eq(2)
  end

  it 'normalizes GTIN values (strips non-digits)' do
    write_ampp_xml([{ appid: '666', nm: 'Padded GTIN Product' }])
    write_single_gtin_xml(amppid: '666', gtin: '05016298210989', startdt: '2020-01-01')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(barcode_record('05016298210989')).to be_present
  end

  it 'updates existing records on re-import' do
    NhsDmdBarcode.create!(
      gtin: '5016298210989', code: 'old', display: 'Old Name',
      system: 'https://dmd.nhs.uk', concept_class: 'AMPP'
    )

    write_ampp_xml([{ appid: '777', nm: 'Updated Name' }])
    write_single_gtin_xml(amppid: '777', gtin: '5016298210989', startdt: '2020-01-01')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(barcode_record('5016298210989')).to have_attributes(
      code: '777',
      display: 'Updated Name'
    )
  end

  it 'raises when AMPP XML is missing' do
    write_gtin_xml([{ amppid: '111', gtins: [] }])

    expect { importer.import(release_dir) }
      .to raise_error(ArgumentError, /No file matching f_ampp2_3/)
  end

  it 'raises when GTIN XML and ZIP are both missing' do
    write_ampp_xml([{ appid: '111', nm: 'Test' }])

    expect { importer.import(release_dir) }
      .to raise_error(ArgumentError, /No GTIN XML or ZIP found/)
  end
end
