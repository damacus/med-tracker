# frozen_string_literal: true

class PaidFeature
  ENV_FLAGS = {
    ai_medication_help: 'MEDTRACKER_AI_MEDICATION_HELP_ENABLED'
  }.freeze

  TRUE_VALUES = %w[1 true yes on].freeze

  def self.enabled?(feature, user: nil)
    new(user: user).enabled?(feature)
  end

  def initialize(user: nil)
    @user = user
  end

  def enabled?(feature)
    flag = ENV_FLAGS[feature.to_sym]
    return false if flag.blank?

    TRUE_VALUES.include?(ENV.fetch(flag, 'false').to_s.downcase)
  end
end
