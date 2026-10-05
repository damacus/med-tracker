# frozen_string_literal: true

module Audit
  class VersionEvent
    class << self
      def record!(**attributes)
        audit_context = audit_context_for(attributes)
        PaperTrail::Version.insert( # rubocop:disable Rails/SkipsModelValidations
          version_attributes(attributes, audit_context)
        )
      end

      private

      def audit_context_for(attributes)
        direct_context = attributes.slice(:whodunnit, :ip, :request_id, :household_id, :actor_membership_id)
        Context.merge(attributes.delete(:context).to_h.merge(direct_context).compact)
      end

      def version_attributes(attributes, audit_context)
        {
          item_type: attributes.fetch(:item_type),
          item_id: attributes.fetch(:item_id),
          event: attributes.fetch(:event),
          object: serialized_object(attributes.fetch(:object)),
          whodunnit: audit_context[:actor_user_id]&.to_s,
          ip: audit_context[:ip],
          request_id: audit_context[:request_id],
          household_id: audit_context[:household_id],
          actor_membership_id: audit_context[:actor_membership_id],
          audit_context: audit_context,
          created_at: attributes.fetch(:created_at, Time.current)
        }
      end

      def serialized_object(object)
        object.is_a?(String) ? object : object.to_json
      end
    end
  end
end
