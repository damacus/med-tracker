# frozen_string_literal: true

module MedTrackerMcp
  class ToolContext
    attr_reader :server_context

    def initialize(server_context)
      @server_context = server_context
    end

    def account
      value(:account)
    end

    def household
      value(:household)
    end

    def membership
      value(:membership)
    end

    def request_id
      value(:request_id)
    end

    def remote_ip
      value(:remote_ip)
    end

    def with_current(&)
      TenantContext.with(account: account, household: household, membership: membership, request_id: request_id, &)
    end

    def policy_scope(scope)
      Pundit.policy_scope!(AuthorizationContext.current, scope)
    end

    private

    def value(key)
      server_context[key]
    end
  end
end
