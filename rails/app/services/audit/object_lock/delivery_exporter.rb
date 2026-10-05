# frozen_string_literal: true

module Audit
  module ObjectLock
    class DeliveryExporter
      def initialize(adapter:)
        @adapter = adapter
      end

      def deliver(delivery)
        delivery.with_lock do
          return true if delivery.delivered?

          delivery.update!(attempts: delivery.attempts + 1)
          persist_success(delivery, adapter.write(delivery.export_record))
        end
        true
      rescue RetryableError => e
        persist_failure(delivery, e, status: :pending, code: 'retryable', retry_at: next_retry(delivery.attempts))
        false
      rescue ConfigurationError, IntegrityError => e
        persist_failure(delivery, e, status: :failed, code: error_code(e))
        false
      end

      private

      attr_reader :adapter

      def persist_success(delivery, result)
        delivery.update!(
          status: :delivered, object_key: result.object_key, checksum_sha256: result.checksum_sha256,
          object_version_id: result.version_id, retention_mode: result.retention_mode,
          retain_until: result.retain_until, delivered_at: Time.current,
          next_attempt_at: nil, last_error_code: nil, last_error_message: nil
        )
      end

      def persist_failure(delivery, error, status:, code:, retry_at: nil)
        delivery.reload
        delivery.update!(
          status:, attempts: delivery.attempts + 1, next_attempt_at: retry_at, last_error_code: code,
          last_error_message: error.message.to_s.first(500)
        )
      end

      def next_retry(attempts)
        Time.current + [2**attempts, 300].min.seconds
      end

      def error_code(error)
        error.is_a?(ConfigurationError) ? 'configuration' : 'integrity'
      end
    end
  end
end
