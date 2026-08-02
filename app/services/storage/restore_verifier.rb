# frozen_string_literal: true

module Storage
  class RestoreVerifier
    class VerificationError < StandardError; end

    Result = Data.define(
      :attachment_id,
      :blob_id,
      :byte_size,
      :required_backends,
      :authorized_retrieval,
      :cross_household_denied
    )

    def self.call(attachment_id: nil, **)
      new(attachment_id:, **).call
    end

    def initialize(
      attachment_id: nil,
      service_registry: ActiveStorage::Blob.services,
      access_verifier: nil
    )
      @attachment_id = attachment_id
      @service_registry = service_registry
      @access_verifier = access_verifier
    end

    def call
      attachment = restored_attachment
      blob = attachment.blob
      verify_stored_object!(blob)
      access = verify_access!(attachment)

      Result.new(
        attachment_id: attachment.id,
        blob_id: blob.id,
        byte_size: blob.byte_size,
        required_backends: RecoverySet.new(blob_scope: ActiveStorage::Blob.where(id: blob.id)).required_backends,
        **access
      )
    rescue ActiveStorage::IntegrityError, ActiveStorage::FileNotFoundError
      raise VerificationError, 'The restored object checksum does not match its database record'
    end

    private

    attr_reader :attachment_id, :service_registry, :access_verifier

    def restored_attachment
      attachment = if attachment_id.present?
                     ActiveStorage::Attachment.find_by(id: attachment_id)
                   else
                     ActiveStorage::Attachment.order(:id).first
                   end
      return attachment if attachment

      raise VerificationError, 'No attachment was found in the restored database'
    end

    def verify_stored_object!(blob)
      service = service_registry.fetch(blob.service_name.to_sym)
      unless service.exist?(blob.key)
        raise VerificationError, 'The restored attachment record exists but its stored object is missing'
      end

      service.open(blob.key, checksum: blob.checksum, verify: true) { it.read(1) }
    rescue KeyError
      raise VerificationError, 'The restored blob service is unavailable'
    end

    def verify_access!(attachment)
      return { authorized_retrieval: nil, cross_household_denied: nil } unless access_verifier

      evidence = access_verifier.call(attachment:)
      raise VerificationError, 'authorized_retrieval_failed' unless evidence[:authorized_retrieval] == true
      raise VerificationError, 'cross_household_denial_failed' unless evidence[:cross_household_denied] == true

      evidence.slice(:authorized_retrieval, :cross_household_denied)
    end
  end
end
