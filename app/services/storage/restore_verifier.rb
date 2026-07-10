# frozen_string_literal: true

module Storage
  class RestoreVerifier
    class VerificationError < StandardError; end

    Result = Data.define(:attachment_id, :blob_id, :byte_size)

    def self.call(attachment_id: nil)
      new(attachment_id: attachment_id).call
    end

    def initialize(attachment_id: nil)
      @attachment_id = attachment_id
    end

    def call
      attachment = restored_attachment
      blob = attachment.blob
      verify_stored_object!(blob)

      Result.new(
        attachment_id: attachment.id,
        blob_id: blob.id,
        byte_size: blob.byte_size
      )
    rescue ActiveStorage::IntegrityError
      raise VerificationError, 'The restored object checksum does not match its database record'
    end

    private

    attr_reader :attachment_id

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
      unless blob.service.exist?(blob.key)
        raise VerificationError, 'The restored attachment record exists but its stored object is missing'
      end

      blob.open { |file| file.read(1) }
    end
  end
end
