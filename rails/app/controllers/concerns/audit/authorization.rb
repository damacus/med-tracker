# frozen_string_literal: true

module Audit
  module Authorization
    def authorize(record, query = nil, policy_class: nil)
      result = super
      resolved_policy_class = policy_class || policy(record).class
      Audit::Context.authorized!(
        policy_class: resolved_policy_class,
        query: query || "#{action_name}?"
      )
      result
    end
  end
end
