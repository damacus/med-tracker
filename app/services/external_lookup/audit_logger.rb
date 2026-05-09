# frozen_string_literal: true

module ExternalLookup
  class AuditLogger
    ITEM_TYPE = 'ExternalMedicineLookup'

    def record(source:, event:, query:, result_status:, result_count: 0)
      # rubocop:disable Rails/SkipsModelValidations
      # PaperTrail::Version is a gem storage table — there are no meaningful validations to run.
      # We use insert to bypass PaperTrail's before_create callbacks, which would attempt to
      # constantize item_type and look up a matching ActiveRecord class.
      PaperTrail::Version.insert(version_attrs(source:, event:, query:, result_status:, result_count:))
      # rubocop:enable Rails/SkipsModelValidations
    rescue StandardError => e
      Rails.logger.error("ExternalLookup::AuditLogger failed: #{e.class}: #{e.message}")
    end

    private

    def version_attrs(source:, event:, query:, result_status:, result_count:)
      {
        item_type: ITEM_TYPE,
        item_id: 0,
        event: "#{source}/#{event}",
        object: { query_hash: Digest::SHA256.hexdigest(query.to_s.strip.downcase),
                  result_status: result_status,
                  result_count: result_count }.to_json,
        whodunnit: PaperTrail.request.whodunnit,
        ip: PaperTrail.request.controller_info&.dig(:ip),
        request_id: PaperTrail.request.controller_info&.dig(:request_id),
        created_at: Time.current
      }
    end
  end
end
