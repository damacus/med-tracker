# frozen_string_literal: true

class Current < ActiveSupport::CurrentAttributes
  attribute :account, :household, :membership, :request_id, :job_id, :observability_context,
            :support_access_session, :audit_context
end
