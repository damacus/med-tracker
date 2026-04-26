# frozen_string_literal: true

module NhsDmd
  class ReleaseArchiveImport
    class Error < StandardError; end

    def initialize(importer: ReleaseImport.new, extractor: ReleaseArchiveExtractor.new)
      @importer = importer
      @extractor = extractor
    end

    def import(uploaded_file_or_path, progress_callback: nil)
      archive_path = resolve_archive_path(uploaded_file_or_path)

      Dir.mktmpdir('nhs-dmd-release') do |release_dir|
        progress_callback&.call(status: :extracting, message: 'Extracting release archive')
        @extractor.extract(archive_path, release_dir)
        @importer.import(release_dir, progress_callback: progress_callback)
      end
    rescue ReleaseArchiveExtractor::Error => e
      raise Error, e.message
    rescue ArgumentError, ActiveRecord::ActiveRecordError => e
      raise Error, "dm+d import failed: #{e.message}"
    end

    private

    def resolve_archive_path(uploaded_file_or_path)
      path = if uploaded_file_or_path.respond_to?(:path)
               uploaded_file_or_path.path
             else
               uploaded_file_or_path.to_s
             end

      raise Error, 'Select an NHS dm+d release ZIP to import.' if path.blank?

      path
    end
  end
end
