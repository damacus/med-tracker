# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'
require 'tmpdir'
require 'zip'

RSpec.describe NhsDmd::ReleaseArchiveImport do
  subject(:service) { described_class.new(importer: importer, extractor: extractor) }

  let(:importer) { instance_double(NhsDmd::ReleaseImport) }
  let(:extractor) { instance_double(NhsDmd::ReleaseArchiveExtractor) }
  let(:result) do
    NhsDmd::ReleaseImport::Result.new(
      created_count: 10,
      updated_count: 2,
      unchanged_count: 0,
      skipped_expired_count: 1,
      skipped_missing_name_count: 1,
      skipped_invalid_count: 1
    )
  end
  let(:uploaded_file) { Struct.new(:path).new('/tmp/release.zip') }

  it 'extracts the uploaded archive into a temp directory before importing it' do
    extracted_dir = nil

    allow(extractor).to receive(:extract) do |_zip_path, destination|
      extracted_dir = destination
    end
    allow(importer).to receive(:import).and_return(result)

    service.import(uploaded_file, progress_callback: ->(_payload) {})

    expect(extractor).to have_received(:extract).with('/tmp/release.zip', extracted_dir)
    expect(importer).to have_received(:import).with(extracted_dir, progress_callback: instance_of(Proc))
  end

  it 'returns the importer result' do
    allow(extractor).to receive(:extract)
    allow(importer).to receive(:import).and_return(result)

    returned_result = service.import(uploaded_file, progress_callback: ->(_payload) {})

    expect(returned_result).to eq(result)
  end

  it 'runs the archive import with the query cache disabled' do
    allow(extractor).to receive(:extract)
    allow(importer).to receive(:import).and_return(result)
    allow(ActiveRecord::Base).to receive(:uncached).and_yield

    service.import(uploaded_file, progress_callback: ->(_payload) {})

    expect(ActiveRecord::Base).to have_received(:uncached)
  end

  describe 'with a real release archive' do
    let(:importer) { NhsDmd::ReleaseImport.new }
    let(:extractor) { NhsDmd::ReleaseArchiveExtractor.new }
    let(:release_root) { Pathname.new(Dir.mktmpdir('release-archive-import-spec', Rails.root.join('tmp'))) }
    let(:uploaded_file) { Struct.new(:path).new(archive_path.to_s) }

    def archive_path
      release_root.join('release.zip')
    end

    def archive_entries
      {
        'f_ampp2_3000000.xml' => '<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS>' \
                                 '<AMPP><APPID>777</APPID><NM>Updated Name</NM></AMPP>' \
                                 '</AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>',
        'f_gtin2_0000000.xml' => '<GTIN_DETAILS><AMPPS>' \
                                 '<AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN>' \
                                 '<STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP>' \
                                 '</AMPPS></GTIN_DETAILS>'
      }
    end

    after { FileUtils.rm_rf(release_root) }

    it 'does not serve barcode lookup selects from the query cache' do
      create_existing_barcode
      write_release_archive
      barcode_selects = []
      subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
        next unless payload[:sql].include?('FROM "nhs_dmd_barcodes"') && payload[:sql].start_with?('SELECT')

        barcode_selects << payload
      end

      ActiveRecord::Base.cache do
        NhsDmdBarcode.find_by(gtin: '5016298210989')
        ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') { service.import(uploaded_file) }
      end

      expect(barcode_selects).not_to be_empty
      expect(barcode_selects).not_to include(include(cached: true))
    end

    def create_existing_barcode
      NhsDmdBarcode.create!(
        gtin: '5016298210989', code: 'old', display: 'Old Name',
        system: 'https://dmd.nhs.uk', concept_class: 'AMPP'
      )
    end

    def write_release_archive
      Zip::File.open(archive_path.to_s, create: true) do |archive|
        archive_entries.each do |name, content|
          archive.get_output_stream(name) { |stream| stream.write(content) }
        end
      end
    end
  end
end
