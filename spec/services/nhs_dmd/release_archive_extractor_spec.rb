# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::ReleaseArchiveExtractor do
  describe '#extract' do
    let(:zip_path) { '/tmp/test_release.zip' }
    let(:destination) { '/tmp/test_extract' }

    # system() is a Kernel method; stub it on the instance via allow_any_instance_of
    # so RSpec/SubjectStub is not triggered (no explicit subject is defined).
    context 'when unzip succeeds' do
      before do
        allow_any_instance_of(described_class).to receive(:system).and_return(true) # rubocop:disable RSpec/AnyInstance
      end

      it 'returns the truthy result of system' do
        result = described_class.new.extract(zip_path, destination)

        expect(result).to be_truthy
      end
    end

    context 'when unzip is not available on the server' do
      before do
        allow_any_instance_of(described_class).to receive(:system).and_raise(Errno::ENOENT) # rubocop:disable RSpec/AnyInstance
      end

      it 'raises ReleaseArchiveExtractor::Error with an explanatory message' do
        expect { described_class.new.extract(zip_path, destination) }
          .to raise_error(described_class::Error, 'ZIP extraction is unavailable on this server.')
      end
    end

    context 'when unzip fails with a generic error' do
      before do
        allow_any_instance_of(described_class).to receive(:system).and_raise(StandardError, 'non-zero exit') # rubocop:disable RSpec/AnyInstance
      end

      it 'raises ReleaseArchiveExtractor::Error wrapping the original message' do
        expect { described_class.new.extract(zip_path, destination) }
          .to raise_error(described_class::Error, /ZIP extraction failed: non-zero exit/)
      end
    end

    context 'with Pathname arguments' do
      let(:zip_path) { Pathname.new('/tmp/release.zip') }
      let(:destination) { Pathname.new('/tmp/extract') }

      before do
        allow_any_instance_of(described_class).to receive(:system).and_return(true) # rubocop:disable RSpec/AnyInstance
      end

      it 'does not raise an error when given Pathname arguments' do
        expect { described_class.new.extract(zip_path, destination) }.not_to raise_error
      end
    end
  end

  describe 'Error' do
    it 'is a subclass of StandardError' do
      expect(described_class::Error.ancestors).to include(StandardError)
    end
  end
end
