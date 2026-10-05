# frozen_string_literal: true

AuthorizationContext = Data.define(:account, :household, :membership) do
  def self.current
    return unless Current.account && Current.household && Current.membership

    new(account: Current.account, household: Current.household, membership: Current.membership)
  end
end
