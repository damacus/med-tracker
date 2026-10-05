# frozen_string_literal: true

class AuthenticationLifetime
  def self.inactivity_days
    configured_integer('SESSION_INACTIVITY_TIMEOUT_DAYS', '30', minimum: 1)
  end

  def self.maximum_age_days
    configured_integer('SESSION_MAX_AGE_DAYS', '0', minimum: 0)
  end

  def self.api_token_maximum_age_months
    configured_integer('API_APP_TOKEN_MAX_AGE_MONTHS', '12', minimum: 1)
  end

  def self.configured_integer(name, default, minimum:)
    value = Integer(ENV.fetch(name, default), 10)
    raise ArgumentError, "#{name} must be at least #{minimum}" if value < minimum

    value
  end
  private_class_method :configured_integer
end
