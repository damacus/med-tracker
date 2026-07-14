# frozen_string_literal: true

require 'fileutils'

module Households
  class HostedExportTransfer
    DEFAULT_OUTPUT_ROOT = Rails.root.join('storage/exports').to_s
    Result = Data.define(:artifact_byte_size, :artifact_checksum_sha256)

    class << self
      def call(export:, actor_account:, destination:)
        new(export: export, actor_account: actor_account, destination: destination).call
      end
    end

    def initialize(export:, actor_account:, destination:)
      @export = export
      @actor_account = actor_account
      @destination = destination
    end

    def call
      path = validated_destination
      reserve(path) { |file| transfer(file) }
    rescue StandardError
      File.delete(path) if @reserved && path && File.exist?(path)
      raise
    end

    private

    def validated_destination
      root = configured_root
      destination = Pathname.new(@destination).expand_path
      parent = destination.dirname
      raise ArgumentError, 'Destination parent directory does not exist' unless parent.directory?

      resolved_parent = parent.realpath
      unless resolved_parent == root || resolved_parent.to_s.start_with?("#{root}#{File::SEPARATOR}")
        raise ArgumentError, 'Destination must be inside the configured export output root'
      end

      resolved_parent.join(destination.basename)
    end

    def configured_root
      root = Pathname.new(ENV.fetch('HOUSEHOLD_EXPORT_OUTPUT_ROOT', DEFAULT_OUTPUT_ROOT)).expand_path
      FileUtils.mkdir_p(root, mode: 0o700)
      root.realpath
    end

    def reserve(path)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL | File::NOFOLLOW, 0o600) do |file|
        @reserved = true
        yield file.binmode
      end
    end

    def transfer(file)
      bytes = HostedExport.download!(export: @export, actor_account: @actor_account)
      checksum = Digest::SHA256.hexdigest(bytes)
      verify_artifact!(bytes, checksum)
      write(file, bytes)
      Result.new(artifact_byte_size: bytes.bytesize, artifact_checksum_sha256: checksum)
    end

    def verify_artifact!(bytes, checksum)
      return if bytes.bytesize == @export.artifact_byte_size && checksum == @export.artifact_checksum_sha256

      raise ActiveStorage::IntegrityError, 'Hosted export artifact verification failed'
    end

    def write(file, bytes)
      file.write(bytes)
      file.flush
      file.fsync
    end
  end
end
