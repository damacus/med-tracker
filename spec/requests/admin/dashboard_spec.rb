# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::Dashboard' do
  fixtures :all

  describe 'GET /admin' do
    it 'returns success for an administrator' do
      sign_in(users(:admin))

      get admin_root_path

      expect(response).to have_http_status(:success)
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
