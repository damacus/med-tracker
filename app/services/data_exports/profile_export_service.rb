# frozen_string_literal: true

require 'base64'
require 'stringio'
require 'zlib'

module DataExports
  class ProfileExportService
    class Error < StandardError; end

    def initialize(household:, membership:, mode:, passphrase: nil, request: nil)
      @household = household
      @membership = membership
      @mode = mode.to_s
      @passphrase = passphrase
      @request = request
    end

    def call
      case mode
      when 'encrypted_migration_bundle'
        encrypted_migration_bundle
      when 'backup_zip'
        backup_zip
      when 'health_data_json'
        health_data_json
      else
        raise Error, 'Export mode is unsupported'
      end
    end

    private

    attr_reader :household, :membership, :mode, :passphrase, :request

    def encrypted_migration_bundle
      raise Error, 'Portable passphrase header is required' if passphrase.blank?

      portable_exporter(passphrase: passphrase).call
    end

    def backup_zip
      json = JSON.pretty_generate(portable_payload.merge(format: 'medtracker.backup.v1'))
      {
        filename: "medtracker-backup-#{Time.current.utc.strftime('%Y%m%d%H%M%S')}.zip",
        content_type: 'application/zip',
        base64: Base64.strict_encode64(zip_file('medtracker-backup.json', json))
      }
    end

    def health_data_json
      portable_payload.merge(format: 'medtracker.health_data.v1')
    end

    def portable_payload
      @portable_payload ||= portable_exporter(passphrase: nil).payload
    end

    def portable_exporter(passphrase:)
      PortableData::Exporter.new(
        household: household,
        membership: membership,
        passphrase: passphrase,
        request: request
      )
    end

    def zip_file(filename, content)
      io = StringIO.new.binmode
      crc = Zlib.crc32(content)
      size = content.bytesize
      name = filename.b

      local_header_offset = write_local_file(io, name, content, crc, size)
      central_directory_offset = io.pos
      write_central_directory(io, name, crc, size, local_header_offset)
      write_end_of_central_directory(io, central_directory_offset)
      io.string
    end

    def write_local_file(io, name, content, crc, size)
      offset = io.pos
      io.write([0x04034b50, 20, 0, 0, 0, 0, crc, size, size, name.bytesize, 0].pack('VvvvvvVVVvv'))
      io.write(name)
      io.write(content)
      offset
    end

    def write_central_directory(io, name, crc, size, local_header_offset)
      central_directory = [
        0x02014b50, 20, 20, 0, 0, 0, 0, crc, size, size, name.bytesize, 0, 0, 0, 0, 0, local_header_offset
      ].pack('VvvvvvvVVVvvvvvVV')
      io.write(central_directory)
      io.write(name)
    end

    def write_end_of_central_directory(io, central_directory_offset)
      central_directory_size = io.pos - central_directory_offset
      payload = [0x06054b50, 0, 0, 1, 1, central_directory_size, central_directory_offset, 0]
      io.write(payload.pack('VvvvvVVv'))
    end
  end
end
