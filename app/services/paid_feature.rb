# frozen_string_literal: true

class PaidFeature
  ENV_FLAGS = {
    ai_medication_help: 'MEDTRACKER_AI_MEDICATION_HELP_ENABLED'
  }.freeze

  PLAN_FEATURES = {
    'family_plus' => %i[ai_medication_help]
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

    household_entitled?(feature)
  end

  private

  attr_reader :user

  def global_flag_enabled?(flag)
    TRUE_VALUES.include?(ENV.fetch(flag, 'false').to_s.downcase)
  end

  def household_entitled?(feature)
    plan = Current.household&.subscription_plan || user_household_subscription_plan
    return false unless plan

    PLAN_FEATURES.fetch(plan, []).include?(feature)
  end

  def user_household_subscription_plan
    person = user&.person
    return person.household.subscription_plan if person&.household

    account = person&.account
    account&.first_active_household&.subscription_plan
  end
end
