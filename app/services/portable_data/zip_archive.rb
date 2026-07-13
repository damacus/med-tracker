# frozen_string_literal: true

require 'stringio'
require 'zlib'

module PortableData
  class ZipArchive
    def self.build(entries)
      new(entries).build
    end

    def initialize(entries)
      @entries = entries
    end

    def build
      io = StringIO.new.binmode
      directory = entries.map { |name, content| write_entry(io, name.to_s.b, content.to_s.b) }
      directory_offset = io.pos
      directory.each { |entry| write_directory_entry(io, entry) }
      write_directory_end(io, directory.size, directory_offset)
      io.string
    end

    private

    attr_reader :entries

    def write_entry(io, name, content)
      crc = Zlib.crc32(content)
      offset = io.pos
      io.write([0x04034b50, 20, 0, 0, 0, 0, crc, content.bytesize, content.bytesize, name.bytesize, 0]
                 .pack('VvvvvvVVVvv'))
      io.write(name)
      io.write(content)
      { name: name, crc: crc, size: content.bytesize, offset: offset }
    end

    def write_directory_entry(io, entry)
      name = entry.fetch(:name)
      io.write([
        0x02014b50, 20, 20, 0, 0, 0, 0, entry.fetch(:crc), entry.fetch(:size), entry.fetch(:size),
        name.bytesize, 0, 0, 0, 0, 0, entry.fetch(:offset)
      ].pack('VvvvvvvVVVvvvvvVV'))
      io.write(name)
    end

    def write_directory_end(io, count, offset)
      size = io.pos - offset
      io.write([0x06054b50, 0, 0, count, count, size, offset, 0].pack('VvvvvVVv'))
    end
  end
end
