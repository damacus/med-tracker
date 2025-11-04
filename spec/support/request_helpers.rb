# frozen_string_literal: true

module RequestHelpers
  def sign_in_as(user)
    # Go through the actual login flow to properly set up the session
    post login_path, params: {
      email_address: user.email_address,
      password: 'password'
    }
    # The session cookie is now set by the controller
  end
end

RSpec.configure do |config|
  config.include RequestHelpers, type: :request
end
