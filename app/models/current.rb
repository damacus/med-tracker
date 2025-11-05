# frozen_string_literal: true

class Current < ActiveSupport::CurrentAttributes
  attribute :session, :request
  delegate :user, to: :session, allow_nil: true
end
