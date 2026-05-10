# frozen_string_literal: true

class PaidFeature
  ENV_FLAGS = {
    ai_medication_help: "MEDTRACKER_AI_MEDICATION_HELP_ENABLED"
  }.freeze

  PLAN_FEATURES = {
    "family_plus" => %i[ai_medication_help]
  }.freeze

  TRUE_VALUES = %w[1 true yes on].freeze

  def self.enabled?(feature, user: nil)
    new(user: user).enabled?(feature)
  end

  def initialize(user: nil)
    @user = user
  end

  def enabled?(feature)
    feature = feature.to_sym
    flag = ENV_FLAGS[feature]
    return false if flag.blank?
    return false unless global_flag_enabled?(flag)

    account_entitled?(feature)
  end

  private

  attr_reader :user

  def global_flag_enabled?(flag)
    TRUE_VALUES.include?(ENV.fetch(flag, "false").to_s.downcase)
  end

  def account_entitled?(feature)
    account = user&.person&.account
    return false unless account

    PLAN_FEATURES.fetch(account.subscription_plan, []).include?(feature)
  end
end
