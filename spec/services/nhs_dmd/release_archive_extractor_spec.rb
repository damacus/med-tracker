# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::ReleaseArchiveExtractor do
  subject(:extractor) { described_class.new }

  describe '#extract' do
    let(:zip_path) { '/tmp/test_release.zip' }
    let(:destination) { '/tmp/test_extract' }

    context 'when unzip succeeds' do
      before do
        allow(extractor).to receive(:system).with('unzip', '-o', zip_path, '-d', destination,
                                                   exception: true).and_return(true)
      end

      it 'calls unzip with the correct arguments' do
        extractor.extract(zip_path, destination)

        expect(extractor).to have_received(:system).with('unzip', '-o', zip_path, '-d', destination,
                                                         exception: true)
      end

      it 'returns the truthy result of system' do
        result = extractor.extract(zip_path, destination)

        expect(result).to be_truthy
      end
    end

    context 'when unzip is not available on the server' do
      before do
        allow(extractor).to receive(:system).and_raise(Errno::ENOENT)
      end

      it 'raises ReleaseArchiveExtractor::Error with an explanatory message' do
        expect { extractor.extract(zip_path, destination) }
          .to raise_error(described_class::Error, 'ZIP extraction is unavailable on this server.')
      end
    end

    context 'when unzip fails with a generic error' do
      before do
        allow(extractor).to receive(:system).and_raise(StandardError, 'non-zero exit')
      end

      it 'raises ReleaseArchiveExtractor::Error wrapping the original message' do
        expect { extractor.extract(zip_path, destination) }
          .to raise_error(described_class::Error, /ZIP extraction failed: non-zero exit/)
      end
    end

    context 'with Pathname arguments' do
      let(:zip_path) { Pathname.new('/tmp/release.zip') }
      let(:destination) { Pathname.new('/tmp/extract') }

      before do
        allow(extractor).to receive(:system).with('unzip', '-o', zip_path.to_s, '-d', destination.to_s,
                                                   exception: true).and_return(true)
      end

      it 'converts Pathname arguments to strings' do
        extractor.extract(zip_path, destination)

        expect(extractor).to have_received(:system).with('unzip', '-o', zip_path.to_s, '-d', destination.to_s,
                                                         exception: true)
      end
    end
  end

  describe 'Error' do
    it 'is a subclass of StandardError' do
      expect(described_class::Error.ancestors).to include(StandardError)
    end
  end
end
