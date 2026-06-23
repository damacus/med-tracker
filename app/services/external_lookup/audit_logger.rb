# frozen_string_literal: true

module ExternalLookup
  class AuditLogger
    ITEM_TYPE = 'ExternalMedicineLookup'

    def record(source:, event:, query:, result_status:, **details)
      # rubocop:disable Rails/SkipsModelValidations
      # PaperTrail::Version is a gem storage table — there are no meaningful validations to run.
      # We use insert to bypass PaperTrail's before_create callbacks, which would attempt to
      # resolve the item type string into a matching ActiveRecord class.
      PaperTrail::Version.insert(version_attrs(source:, event:, query:, result_status:, **details))
      # rubocop:enable Rails/SkipsModelValidations
    rescue StandardError => e
      Rails.logger.error("ExternalLookup::AuditLogger failed: #{e.class}: #{e.message}")
    end

    private

    def version_attrs(source:, event:, query:, result_status:, **details)
      {
        item_type: ITEM_TYPE,
        item_id: 0,
        event: "#{source}/#{event}",
        object: object_attrs(query:, result_status:, **details).to_json,
        whodunnit: PaperTrail.request.whodunnit,
        ip: PaperTrail.request.controller_info&.dig(:ip),
        request_id: PaperTrail.request.controller_info&.dig(:request_id),
        household_id: household_id,
        actor_membership_id: actor_membership_id,
        created_at: Time.current
      }
    end

    def object_attrs(query:, result_status:, **details)
      {
        query: query.to_s,
        query_hash: Digest::SHA256.hexdigest(query.to_s.strip.downcase),
        result_status: result_status,
        result_count: details.fetch(:result_count, 0)
      }.merge(details.fetch(:metadata, {}))
    end

    def household_id
      PaperTrail.request.controller_info&.dig(:household_id) || Current.household&.id
    end

    def actor_membership_id
      PaperTrail.request.controller_info&.dig(:actor_membership_id) || Current.membership&.id
    end
  end
end
