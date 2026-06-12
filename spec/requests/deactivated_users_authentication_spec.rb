# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Deactivated user authentication' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }
  let(:account) { accounts(:jane_doe) }

  it 'does not establish a Rodauth session for an inactive user' do
    user.deactivate!

    post '/login', params: { email: account.email, password: 'password' }

    expect(session[:account_id]).to be_blank
    expect(response).not_to redirect_to(dashboard_path)
  end

  it 'revokes access for an inactive user with an existing session' do
    sign_in(user)
    expect(session[:account_id]).to be_present

    user.deactivate!
    get dashboard_path

    expect(response).to redirect_to(login_path)
    expect(session[:account_id]).to be_blank
  end
end
