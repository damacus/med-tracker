# frozen_string_literal: true

module RodauthHelpers
  # Rodauth path helpers for tests
  def login_path
    '/login'
  end

  def logout_path
    '/logout'
  end

  def create_account_path
    '/create-account'
  end

  def verify_account_path
    '/verify-account'
  end

  def reset_password_path
    '/reset-password'
  end
end

RSpec.configure do |config|
  config.include RodauthHelpers, type: :system
  config.include RodauthHelpers, type: :request
end
