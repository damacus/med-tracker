# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::ReleaseArchiveImport do
  subject(:service) { described_class.new(importer: importer, extractor: extractor) }

  let(:importer) { instance_double(NhsDmd::ReleaseImport) }
  let(:extractor) { instance_double(NhsDmd::ReleaseArchiveExtractor) }
  let(:result) { NhsDmd::ReleaseImport::Result.new(imported_count: 12, skipped_count: 3) }
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
end
