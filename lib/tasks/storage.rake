# frozen_string_literal: true

namespace :med_tracker do
  namespace :storage do
    desc 'Verify an attachment restored from the database and persistent storage backup'
    task verify_restore: :environment do
      result = Storage::RestoreVerifier.call(attachment_id: ENV['ATTACHMENT_ID'].presence)
      puts "Storage restore verified: attachment_id=#{result.attachment_id} " \
           "blob_id=#{result.blob_id} byte_size=#{result.byte_size}"
    rescue Storage::RestoreVerifier::VerificationError => e
      abort "Storage restore verification failed: #{e.message}"
    end
  end
end
