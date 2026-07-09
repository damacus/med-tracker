# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'
require 'tmpdir'
require 'zip'
require 'zlib'

RSpec.describe NhsDmd::ReleaseArchiveExtractor do
  describe '#extract' do
    let(:tmp_root) { Pathname.new(Dir.mktmpdir('release-extractor-spec', Rails.root.join('tmp'))) }
    let(:zip_path) { tmp_root.join('release.zip') }
    let(:destination) { tmp_root.join('extract') }

    after { FileUtils.rm_rf(tmp_root) }

    it 'extracts regular files inside the destination' do
      write_zip('f_ampp2_3000000.xml' => '<AMPP />', 'nested/f_gtin2_0000000.xml' => '<GTIN />')

      described_class.new.extract(zip_path, destination)

      expect(destination.join('f_ampp2_3000000.xml').read).to eq('<AMPP />')
      expect(destination.join('nested/f_gtin2_0000000.xml').read).to eq('<GTIN />')
    end

    it 'rejects traversal entries before writing files' do
      write_zip('../escape.txt' => 'owned', 'f_ampp2_3000000.xml' => '<AMPP />')

      expect { described_class.new.extract(zip_path, destination) }
        .to raise_error(described_class::Error, /unsafe ZIP entry/)

      expect(tmp_root.join('escape.txt')).not_to exist
      expect(destination).not_to exist
    end

    it 'rejects absolute entries before writing files' do
      write_raw_zip_entry('/tmp/absolute.txt', 'owned')

      expect { described_class.new.extract(zip_path, destination) }
        .to raise_error(described_class::Error, /unsafe ZIP entry/)
    end

    it 'rejects oversized entries before writing files' do
      stub_const("#{described_class}::MAX_ENTRY_BYTES", 4)
      write_zip('f_ampp2_3000000.xml' => 'too-large')

      expect { described_class.new.extract(zip_path, destination) }
        .to raise_error(described_class::Error, /too large/)
    end

    it 'extracts only matching entries when a pattern is provided' do
      write_zip('f_ampp2_3000000.xml' => '<AMPP />', 'f_gtin2_0000000.xml' => '<GTIN />')

      described_class.new.extract(zip_path, destination, pattern: 'f_gtin2_0*.xml')

      expect(destination.join('f_gtin2_0000000.xml')).to exist
      expect(destination.join('f_ampp2_3000000.xml')).not_to exist
    end
  end

  describe 'Error' do
    it 'is a subclass of StandardError' do
      expect(described_class::Error.ancestors).to include(StandardError)
    end
  end

  def write_zip(entries)
    Zip::File.open(zip_path.to_s, create: true) do |zip|
      entries.each do |name, content|
        zip.get_output_stream(name) { |io| io.write(content) }
      end
    end
  end

  def write_raw_zip_entry(name, content)
    File.binwrite(zip_path, raw_zip_entry(name, content))
  end

  def raw_zip_entry(name, content)
    encoded_name = name.b
    body = content.b
    local_header = raw_zip_local_header(encoded_name, body)
    central_header = raw_zip_central_header(encoded_name, body)
    end_record = raw_zip_end_record(encoded_name, body, local_header, central_header)

    local_header + encoded_name + body + central_header + encoded_name + end_record
  end

  def raw_zip_local_header(encoded_name, body)
    crc = Zlib.crc32(body)
    [
      0x04034b50, 20, 0, 0, 0, 0, crc, body.bytesize, body.bytesize, encoded_name.bytesize, 0
    ].pack('VvvvvvVVVvv')
  end

  def raw_zip_central_header(encoded_name, body)
    crc = Zlib.crc32(body)
    [
      0x02014b50, 20, 20, 0, 0, 0, 0, crc, body.bytesize, body.bytesize, encoded_name.bytesize, 0, 0, 0, 0, 0,
      0
    ].pack('VvvvvvvVVVvvvvvVV')
  end

  def raw_zip_end_record(encoded_name, body, local_header, central_header)
    central_header_offset = local_header.bytesize + encoded_name.bytesize + body.bytesize
    central_header_size = central_header.bytesize + encoded_name.bytesize

    [0x06054b50, 0, 0, 1, 1, central_header_size, central_header_offset, 0].pack('VvvvvVVv')
  end
end
