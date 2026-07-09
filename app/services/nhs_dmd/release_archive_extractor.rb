# frozen_string_literal: true

require 'fileutils'
require 'zip'

module NhsDmd
  class ReleaseArchiveExtractor
    class Error < StandardError; end

    MAX_ENTRIES = 200
    MAX_ENTRY_BYTES = 100.megabytes
    MAX_TOTAL_BYTES = 500.megabytes

    def extract(zip_path, destination, pattern: nil)
      destination = Pathname.new(destination).expand_path
      entries = validated_entries(zip_path, destination, pattern)
      FileUtils.mkdir_p(destination.to_s)
      Zip::File.open(zip_path.to_s) do |zip_file|
        entries.each do |entry_name|
          extract_entry(zip_file.find_entry(entry_name), destination)
        end
      end
    rescue Error
      raise
    rescue Zip::Error, SystemCallError => e
      raise Error, "ZIP extraction failed: #{e.message}"
    end

    private

    def validated_entries(zip_path, destination, pattern)
      total_size = 0
      entries = []
      Zip::File.open(zip_path.to_s) do |zip_file|
        zip_file.each do |entry|
          next if pattern && !File.fnmatch?(pattern, entry.name, File::FNM_PATHNAME)

          validate_entry!(entry, destination)
          total_size += entry.size.to_i
          raise Error, 'ZIP extraction would exceed expanded size limit.' if total_size > MAX_TOTAL_BYTES

          entries << entry.name
          raise Error, 'ZIP extraction contains too many entries.' if entries.size > MAX_ENTRIES
        end
      end
      entries
    end

    def validate_entry!(entry, destination)
      validate_entry_name!(entry)
      validate_entry_size!(entry)
      validate_entry_target!(entry, destination)
    end

    def validate_entry_name!(entry)
      return unless unsafe_name?(entry.name) || symlink_entry?(entry)

      raise Error, "unsafe ZIP entry: #{entry.name}"
    end

    def validate_entry_size!(entry)
      return unless entry.size.to_i > MAX_ENTRY_BYTES

      raise Error, "ZIP entry is too large: #{entry.name}"
    end

    def validate_entry_target!(entry, destination)
      return if safe_target_path?(target_path(destination, entry.name), destination)

      raise Error, "unsafe ZIP entry: #{entry.name}"
    end

    def safe_target_path?(target, destination)
      target == destination || target.to_s.start_with?("#{destination}/")
    end

    def target_path(destination, name)
      destination.join(name).cleanpath.expand_path
    end

    def unsafe_name?(name)
      path = Pathname.new(name)
      path.absolute? || path.each_filename.any? { |part| part == '..' }
    end

    def symlink_entry?(entry)
      return entry.symlink? if entry.respond_to?(:symlink?)

      entry.respond_to?(:ftype) && entry.ftype == :symlink
    end

    def extract_entry(entry, destination)
      target = target_path(destination, entry.name)
      if entry.directory?
        FileUtils.mkdir_p(target.to_s)
      else
        FileUtils.mkdir_p(target.dirname.to_s)
        entry.extract(entry.name, destination_directory: destination.to_s) { true }
      end
    end
  end
end
