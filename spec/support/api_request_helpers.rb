# frozen_string_literal: true

module ApiRequestHelpers
  def api_login(user, password: 'password', device_name: 'RSpec iPhone')
    post api_v1_auth_login_path,
         params: {
           email: user.email_address,
           password: password,
           device_name: device_name
         },
         as: :json

    response.parsed_body.fetch('data')
  end

  def api_auth_headers(access_token)
    {
      'Authorization' => "Bearer #{access_token}",
      'Accept' => 'application/json'
    }
  end
end

RSpec.configure do |config|
  config.include ApiRequestHelpers, type: :request
end
