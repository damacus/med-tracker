# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::Dashboard' do
  fixtures :all

  describe 'GET /admin' do
    it 'returns success for an administrator' do
      previous_value = ENV.fetch('HOSTED_ADMIN_MFA_REQUIRED', nil)
      ENV.delete('HOSTED_ADMIN_MFA_REQUIRED')
      sign_in(users(:admin))

      get admin_root_path

      expect(response).to have_http_status(:success)
    ensure
      if previous_value.nil?
        ENV.delete('HOSTED_ADMIN_MFA_REQUIRED')
      else
        ENV['HOSTED_ADMIN_MFA_REQUIRED'] = previous_value
      end
    end

    it 'denies a non-administrator' do
      sign_in(users(:jane))

      get admin_root_path

      expect(response).to redirect_to(root_path)
    end

    it 'redirects an unauthenticated visitor to login' do
      get admin_root_path

      expect(response).to redirect_to(login_path)
    end
  end
end
