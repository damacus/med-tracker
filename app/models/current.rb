# frozen_string_literal: true

class Current < ActiveSupport::CurrentAttributes
  attribute :account, :household, :membership, :request_id, :support_access_session, :audit_context
end
