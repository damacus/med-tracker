# frozen_string_literal: true

module Audit
  class Event
    class << self
      def record!(**attributes)
        context = Context.merge(attributes.delete(:audit_context).to_h)
        SecurityAuditEvent.create!(event_attributes(attributes, context))
      end

      private

      def event_attributes(attributes, context)
        request = attributes.delete(:request)
        identity_attributes(attributes).merge(
          event_type: attributes.fetch(:event_type),
          request_id: request&.request_id || attributes[:request_id] || context[:request_id],
          ip: request&.remote_ip || attributes[:ip] || context[:ip],
          metadata: Redactor.call(attributes.fetch(:metadata, {})),
          audit_context: context
        )
      end

      def identity_attributes(attributes)
        {
          household: attributes[:household],
          household_id: attributes[:household_id],
          actor_account: attributes[:actor_account] || Current.account,
          actor_account_id: attributes[:actor_account_id],
          actor_membership: attributes[:actor_membership] || Current.membership,
          actor_membership_id: attributes[:actor_membership_id]
        }.compact
      end
    end
  end
end
