# frozen_string_literal: true

module Api
  module V1
    module Admin
      class AuditLogsController < BaseController
        def index
          events = SecurityAuditEvent.where(household: current_household).order(created_at: :desc).limit(100)
          render json: { data: events.map { |event| audit_event_payload(event) } }
        end

        private

        def audit_event_payload(event)
          {
            id: event.id,
            event_type: event.event_type,
            actor_account_id: event.actor_account_id,
            actor_membership_id: event.actor_membership_id,
            request_id: event.request_id,
            metadata: event.metadata,
            created_at: event.created_at.iso8601
          }
        end
      end
    end
  end
end
