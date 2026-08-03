# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'
require 'tmpdir'
require 'zip'

RSpec.describe NhsDmd::ReleaseImport do
  let(:importer) { described_class.new }
  let(:release_dir) { Pathname.new(Dir.mktmpdir('dmd-release-spec', Rails.root.join('tmp'))) }

  after { FileUtils.rm_rf(release_dir) }

  def write_ampp_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS>'
    entries.each do |e|
      xml << "<AMPP><APPID>#{e[:appid]}</APPID>"
      xml << "<APID>#{e[:apid]}</APID>" if e[:apid]
      xml << "<NM>#{e[:nm]}</NM></AMPP>"
    end
    xml << '</AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>'
    File.write(release_dir.join('f_ampp2_3000000.xml'), xml)
  end

  def write_trade_family_group_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<TRADE_FAMILY_GROUPS>'
    entries.each do |entry|
      xml << "<TRADE_FAMILY_GROUP><TFGID>#{entry[:code]}</TFGID><NM>#{entry[:name]}</NM></TRADE_FAMILY_GROUP>"
    end
    xml << '</TRADE_FAMILY_GROUPS>'
    File.write(release_dir.join('f_trade_family_group2_0060726.xml'), xml)
  end

  def write_trade_family_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<TRADE_FAMILIES>'
    entries.each do |entry|
      xml << "<TRADE_FAMILY><TFID>#{entry[:code]}</TFID><NM>#{entry[:name]}</NM>"
      xml << "<TFGID>#{entry[:group_code]}</TFGID></TRADE_FAMILY>"
    end
    xml << '</TRADE_FAMILIES>'
    File.write(release_dir.join('f_trade_family2_0060726.xml'), xml)
  end

  def write_amp_trade_family_xml(entries)
    xml = +'<?xml version="1.0" encoding="utf-8" ?>'
    xml << '<AMP_TRADE_FAMILIES>'
    entries.each do |entry|
      xml << amp_trade_family_xml(entry)
    end
    xml << '</AMP_TRADE_FAMILIES>'
    File.write(release_dir.join('f_amp_trade_family2_0060726.xml'), xml)
  end

  def amp_trade_family_xml(entry)
    "<AMP_TRADE_FAMILY><APID>#{entry[:amp_code]}</APID><TFID>#{entry[:trade_family_code]}</TFID>" \
      "#{supplementary_date_xml('STARTDT', entry[:startdt])}" \
      "#{supplementary_date_xml('ENDDT', entry[:enddt])}</AMP_TRADE_FAMILY>"
  end

  def supplementary_date_xml(element, value)
    value ? "<#{element}>#{value}</#{element}>" : ''
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

  def nested_gtin_xml
    gtin_entry = '<GTINDATA><GTIN>5016298210989</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA>'
    "<GTIN_DETAILS><AMPPS><AMPP><AMPPID>111</AMPPID>#{gtin_entry}</AMPP></AMPPS></GTIN_DETAILS>"
  end

  def write_nested_gtin_zip
    nested_zip = release_dir.join('release_GTIN.zip')
    Zip::File.open(nested_zip.to_s, create: true) do |zip|
      zip.get_output_stream('f_gtin2_0000000.xml') { |io| io.write(nested_gtin_xml) }
    end
    nested_zip
  end

  def stub_safe_gtin_extractor(safe_extractor, nested_zip)
    allow(safe_extractor).to receive(:extract) do |zip_path, destination, pattern:|
      expect(zip_path).to eq(nested_zip.to_s)
      expect(pattern).to eq('f_gtin2_0*.xml')

      File.write(Pathname.new(destination).join('f_gtin2_0000000.xml'), nested_gtin_xml)
    end
  end

  def gtin_entry(amppid:, gtins:)
    { amppid: amppid, gtins: gtins }
  end

  def barcode_record(gtin)
    NhsDmdBarcode.find_by!(gtin: gtin)
  end

  def capture_relationship_queries
    queries = []
    subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*, payload|
      queries << payload[:sql] if payload[:sql].include?('"nhs_dmd_ampp_relationships"')
    end
    yield
    queries
  ensure
    ActiveSupport::Notifications.unsubscribe(subscriber) if subscriber
  end

  def write_standard_trade_family_release(group_name:, family_name:)
    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')
    write_trade_family_group_xml([{ code: '900', name: group_name }])
    write_trade_family_xml([{ code: '800', name: family_name, group_code: '900' }])
    write_amp_trade_family_xml([{ amp_code: '222', trade_family_code: '800', startdt: '2026-07-06' }])
  end

  def create_existing_trade_family
    group = NhsDmdTradeFamilyGroup.create!(code: '900', name: 'Old Group')
    family = NhsDmdTradeFamily.create!(code: '800', name: 'Old Family', trade_family_group: group)
    NhsDmdAmpTradeFamily.create!(amp_code: '222', trade_family: family)
    family
  end

  def create_existing_barcode
    NhsDmdBarcode.create!(
      gtin: '5016298210989', amp_code: '222', code: '111', display: 'Paracetamol 500mg tablets (Acme Ltd)',
      vmp_name: 'Paracetamol 500mg tablets', system: 'https://dmd.nhs.uk', concept_class: 'AMPP'
    )
  end

  def expect_barcode_trade_family(name:, group_name: nil)
    expected = { trade_family: { code: '800', name: name } }
    expected[:trade_family_group] = { code: '900', name: group_name } if group_name
    expect(NhsDmd::BarcodeLookup.new.lookup('5016298210989')).to include(expected)
  end

  def progress_ampp_entries
    [
      { appid: '111', nm: 'Product One' },
      { appid: '222', nm: 'Product Two' }
    ]
  end

  def progress_gtin_entries
    [
      gtin_entry(
        amppid: '111',
        gtins: [
          { gtin: '1111111111111', startdt: '2020-01-01' },
          { gtin: '2222222222222', startdt: '2020-01-01' }
        ]
      ),
      gtin_entry(
        amppid: '222',
        gtins: [
          { gtin: '3333333333333', startdt: '2020-01-01' }
        ]
      )
    ]
  end

  def write_progress_fixture
    write_ampp_xml(progress_ampp_entries)
    write_gtin_xml(progress_gtin_entries)
  end

  def expect_progress_update(progress_updates, **expected)
    expect(progress_updates).to include(hash_including(**expected))
  end

  def expect_multi_phase_progress(progress_updates)
    expect_progress_update(progress_updates, status: :counting, total_records: 5, processed_records: 0,
                                             message: 'Counted 2 AMPP records and 3 GTIN records')
    expect_progress_update(progress_updates, status: :importing, total_records: 5, processed_records: 0,
                                             message: 'Starting AMPP name import')
    expect_progress_update(progress_updates, status: :importing, total_records: 5, processed_records: 2,
                                             message: 'Processed 2 AMPP records (2 updated, 0 skipped)')
    expect_progress_update(progress_updates, status: :importing, total_records: 5, processed_records: 2,
                                             message: 'Starting GTIN import')
    expect(progress_updates.last).to include(status: :importing, total_records: 5, processed_records: 5,
                                             imported_count: 3, skipped_count: 0)
  end

  it 'imports active GTINs matched to AMPP names' do
    write_ampp_xml([{ appid: '111', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(result.skipped_count).to eq(0)
    expect(barcode_record('5016298210989')).to have_attributes(
      code: '111',
      display: 'Paracetamol 500mg tablets (Acme Ltd)',
      vmp_name: 'Paracetamol 500mg tablets',
      system: 'https://dmd.nhs.uk',
      concept_class: 'AMPP'
    )
  end

  it 'imports an active supplementary AMP-to-trade-family mapping and its release freshness' do
    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')
    write_trade_family_group_xml([{ code: '900', name: 'Acme' }])
    write_trade_family_xml([{ code: '800', name: 'Acme Paracetamol', group_code: '900' }])
    write_amp_trade_family_xml([{ amp_code: '222', trade_family_code: '800', startdt: '2026-07-06' }])

    importer.import(release_dir)

    expect(barcode_record('5016298210989')).to have_attributes(amp_code: '222')
    expect(NhsDmdTradeFamily.find_by!(code: '800')).to have_attributes(name: 'Acme Paracetamol')
    expect(NhsDmdTradeFamilyGroup.find_by!(code: '900')).to have_attributes(name: 'Acme')
    expect(NhsDmdAmpTradeFamily.find_by!(amp_code: '222')).to have_attributes(
      trade_family: NhsDmdTradeFamily.find_by!(code: '800')
    )
    expect(NhsDmdSupplementaryRelease.current).to have_attributes(released_on: Date.new(2026, 7, 6))
  end

  it 'imports the core AMPP-to-AMP relationship when the AMPP has no GTIN row' do
    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_gtin_xml([])

    importer.import(release_dir)

    expect(NhsDmdAmppRelationship.find_by!(ampp_code: '111')).to have_attributes(amp_code: '222')
    expect(NhsDmdBarcode.find_by(code: '111')).to be_nil
  end

  it 'bulk inserts core AMPP-to-AMP relationships without uniqueness lookups' do
    write_ampp_xml(
      [
        { appid: '111', apid: '222', nm: 'First product' },
        { appid: '333', apid: '444', nm: 'Second product' }
      ]
    )
    write_gtin_xml([])
    relationship_queries = capture_relationship_queries { importer.import(release_dir) }

    insert_queries = relationship_queries.grep(/\AINSERT INTO "nhs_dmd_ampp_relationships"/)
    uniqueness_queries = relationship_queries.grep(/\ASELECT .*"nhs_dmd_ampp_relationships"/)

    expect(insert_queries.size).to eq(1)
    expect(uniqueness_queries).to be_empty
    expect(NhsDmdAmppRelationship.order(:ampp_code).pluck(:ampp_code, :amp_code)).to eq(
      [%w[111 222], %w[333 444]]
    )
  end

  it 'tolerates missing supplementary XML and does not retain inactive AMP mappings' do
    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')

    expect { importer.import(release_dir) }.not_to raise_error
    expect(NhsDmdAmpTradeFamily.find_by(amp_code: '222')).to be_nil

    write_trade_family_group_xml([{ code: '900', name: 'Acme' }])
    write_trade_family_xml([{ code: '800', name: 'Acme Paracetamol', group_code: '900' }])
    write_amp_trade_family_xml(
      [{ amp_code: '222', trade_family_code: '800', startdt: '2026-01-01', enddt: '2026-07-05' }]
    )

    importer.import(release_dir)

    expect(NhsDmdAmpTradeFamily.find_by(amp_code: '222')).to be_nil
  end

  it 'keeps existing mappings when supplementary input is incomplete' do
    old_group = NhsDmdTradeFamilyGroup.create!(code: '900', name: 'Old Group')
    old_family = NhsDmdTradeFamily.create!(code: '800', name: 'Old Family', trade_family_group: old_group)
    NhsDmdAmpTradeFamily.create!(amp_code: '222', trade_family: old_family)

    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')
    write_amp_trade_family_xml([{ amp_code: '333', trade_family_code: '800', startdt: '2026-07-06' }])

    expect { importer.import(release_dir) }.not_to raise_error
    expect(NhsDmdAmpTradeFamily.find_by!(amp_code: '222')).to have_attributes(trade_family: old_family)
    expect(barcode_record('5016298210989')).to have_attributes(amp_code: '222')
  end

  it 'expires local barcode lookup caches after replacing supplementary metadata' do
    cache_store = ActiveSupport::Cache::MemoryStore.new
    allow(Rails).to receive(:cache).and_return(cache_store)

    create_existing_trade_family
    create_existing_barcode
    expect_barcode_trade_family(name: 'Old Family')
    write_standard_trade_family_release(group_name: 'New Group', family_name: 'New Family')

    importer.import(release_dir)

    expect_barcode_trade_family(name: 'New Family', group_name: 'New Group')
  end

  it 'continues the core GTIN import when supplementary cache invalidation fails' do
    cache_store = ActiveSupport::Cache::MemoryStore.new
    allow(Rails).to receive(:cache).and_return(cache_store)
    allow(cache_store).to receive(:delete_matched).and_raise(NotImplementedError, 'cache unavailable')
    allow(Observability::DiagnosticEvent).to receive(:emit)

    write_standard_trade_family_release(group_name: 'Acme', family_name: 'Acme Paracetamol')

    result = importer.import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(barcode_record('5016298210989')).to have_attributes(amp_code: '222')
    expect(NhsDmdAmpTradeFamily.find_by!(amp_code: '222')).to have_attributes(
      trade_family: NhsDmdTradeFamily.find_by!(code: '800')
    )
    expect_supplementary_failure_diagnostic
  end

  it 'keeps supplementary metadata when any expected supplementary file has multiple matches' do
    old_family = create_existing_trade_family
    write_standard_trade_family_release(group_name: 'New Group', family_name: 'New Family')

    %w[f_trade_family_group2_0060726.xml f_trade_family2_0060726.xml f_amp_trade_family2_0060726.xml].each do |filename|
      FileUtils.cp(release_dir.join(filename), release_dir.join(filename.sub('060726', '130726')))

      expect { importer.import(release_dir) }.not_to raise_error
      expect(NhsDmdAmpTradeFamily.find_by!(amp_code: '222')).to have_attributes(trade_family: old_family)
      expect(NhsDmdTradeFamilyGroup.find_by!(code: '900')).to have_attributes(name: 'Old Group')
      expect(NhsDmdTradeFamily.find_by!(code: '800')).to have_attributes(name: 'Old Family')
      expect(barcode_record('5016298210989')).to have_attributes(amp_code: '222')

      FileUtils.rm(release_dir.join(filename.sub('060726', '130726')))
    end
  end

  it 'skips memberships with malformed supplied dates' do
    write_ampp_xml([{ appid: '111', apid: '222', nm: 'Paracetamol 500mg tablets (Acme Ltd)' }])
    write_single_gtin_xml(amppid: '111', gtin: '5016298210989', startdt: '2020-01-01')
    write_trade_family_group_xml([{ code: '900', name: 'Acme' }])
    write_trade_family_xml([{ code: '800', name: 'Acme Paracetamol', group_code: '900' }])
    write_amp_trade_family_xml(
      [
        { amp_code: '222', trade_family_code: '800', startdt: 'not-a-date' },
        { amp_code: '333', trade_family_code: '800', startdt: '2026-01-01', enddt: 'not-a-date' }
      ]
    )

    importer.import(release_dir)

    expect(NhsDmdAmpTradeFamily.where(amp_code: %w[222 333])).to be_empty
  end

  it 'stores nil vmp_name when the AMPP name has no manufacturer suffix' do
    write_ampp_xml([{ appid: '222', nm: 'Paracetamol 500mg tablets' }])
    write_single_gtin_xml(amppid: '222', gtin: '5016298210000', startdt: '2020-01-01')

    importer.import(release_dir)

    expect(barcode_record('5016298210000')).to have_attributes(
      display: 'Paracetamol 500mg tablets',
      vmp_name: nil
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
    expect(result.updated_count).to eq(1)
    expect(result.created_count).to eq(0)
    expect(result.unchanged_count).to eq(0)
    expect(barcode_record('5016298210989')).to have_attributes(
      code: '777',
      display: 'Updated Name'
    )
  end

  it 'does not serve barcode lookup selects from the query cache during an archive import' do
    NhsDmdBarcode.create!(
      gtin: '5016298210989', code: 'old', display: 'Old Name',
      system: 'https://dmd.nhs.uk', concept_class: 'AMPP'
    )
    write_ampp_xml([{ appid: '777', nm: 'Updated Name' }])
    write_single_gtin_xml(amppid: '777', gtin: '5016298210989', startdt: '2020-01-01')
    barcode_selects = []
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next unless payload[:sql].include?('FROM "nhs_dmd_barcodes"') && payload[:sql].start_with?('SELECT')

      barcode_selects << payload
    end

    ActiveRecord::Base.cache do
      NhsDmdBarcode.find_by(gtin: '5016298210989')
      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') { importer.import(release_dir) }
    end

    expect(barcode_selects).not_to be_empty
    expect(barcode_selects).not_to include(include(cached: true))
  end

  it 'reports unchanged records when re-importing the same release' do
    write_ampp_xml([{ appid: '888', nm: 'Stable Product' }])
    write_single_gtin_xml(amppid: '888', gtin: '4444444444444', startdt: '2020-01-01')

    first = importer.import(release_dir)
    second = described_class.new.import(release_dir)

    expect(first).to have_attributes(created_count: 1, unchanged_count: 0)
    expect(second).to have_attributes(created_count: 0, updated_count: 0, unchanged_count: 1, imported_count: 0)
  end

  it 'categorises skip reasons separately' do
    write_ampp_xml([{ appid: 'NAMED', nm: 'Known Product' }])
    write_gtin_xml(
      [
        gtin_entry(amppid: 'NAMED', gtins: [{ gtin: '', startdt: '2020-01-01' }]),
        gtin_entry(amppid: 'NAMED',
                   gtins: [{ gtin: '5555555555555', startdt: '2019-01-01', enddt: '2019-12-31' }]),
        gtin_entry(amppid: 'UNKNOWN', gtins: [{ gtin: '6666666666666', startdt: '2020-01-01' }])
      ]
    )

    result = importer.import(release_dir)

    expect(result.skipped_invalid_count).to eq(1)
    expect(result.skipped_expired_count).to eq(1)
    expect(result.skipped_missing_name_count).to eq(1)
    expect(result.skipped_count).to eq(3)
    expect(result.imported_count).to eq(0)
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

  it 'extracts nested GTIN ZIPs through the configured safe archive extractor' do
    write_ampp_xml([{ appid: '111', nm: 'Nested GTIN Product' }])
    safe_extractor = instance_double(NhsDmd::ReleaseArchiveExtractor)
    stub_safe_gtin_extractor(safe_extractor, write_nested_gtin_zip)

    result = described_class.new(extractor: safe_extractor).import(release_dir)

    expect(result.imported_count).to eq(1)
    expect(barcode_record('5016298210989')).to have_attributes(display: 'Nested GTIN Product')
    expect(safe_extractor).to have_received(:extract)
  end

  it 'reports progress across both AMPP parsing and GTIN import work' do
    write_progress_fixture
    progress_updates = []

    importer.import(release_dir, progress_callback: ->(payload) { progress_updates << payload })

    expect_multi_phase_progress(progress_updates)
  end

  def expect_supplementary_failure_diagnostic
    expect(Observability::DiagnosticEvent).to have_received(:emit).with(
      component: :nhs_dmd_supplementary,
      reason: :operation_failed,
      severity: :warn,
      error: instance_of(NotImplementedError)
    )
  end
end
