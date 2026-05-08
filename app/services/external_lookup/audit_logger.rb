# frozen_string_literal: true

module ExternalLookup
  class AuditLogger
    def record(source:, event:, query:, result_status:, result_count: 0)
      ExternalLookupAuditEvent.create!(
        source: source,
        event: event,
        query_hash: Digest::SHA256.hexdigest(query.to_s.strip.downcase),
        result_status: result_status,
        result_count: result_count,
        **request_context
      )
    rescue StandardError => e
      Rails.logger.error("ExternalLookup::AuditLogger failed: #{e.class}: #{e.message}")
    end

    private

    def request_context
      {
        whodunnit: PaperTrail.request.whodunnit,
        ip: PaperTrail.request.controller_info&.dig(:ip),
        request_id: PaperTrail.request.controller_info&.dig(:request_id)
      }
    end
  end
end
