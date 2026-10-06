# frozen_string_literal: true

module Households
  module OperatorAuthorization
    private

    def authorize_operator!(actor_account)
      return if actor_account&.platform_admin&.active?

      raise Pundit::NotAuthorizedError, 'Platform administrator access is required'
    end
  end
end
