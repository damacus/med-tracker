# frozen_string_literal: true

module NhsDmd
  class ReleaseArchiveExtractor
    class Error < StandardError; end

    def extract(zip_path, destination)
      system('unzip', '-o', zip_path.to_s, '-d', destination.to_s, exception: true)
    rescue Errno::ENOENT
      raise Error, 'ZIP extraction is unavailable on this server.'
    rescue StandardError => e
      raise Error, "ZIP extraction failed: #{e.message}"
    end
  end
end
