# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Close account' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }
  let!(:account) { user.person.account }

  before do
    sign_in(user)
  end

  it 'soft deletes the account and blocks future login' do
    post '/close-account', params: { password: 'password' }

    expect(response).to redirect_to('/')
    expect(account.reload).to be_closed
    expect(user.person.reload.account).to be_nil

    get dashboard_path
    expect(response).to redirect_to(login_path)

    post login_path, params: { email: account.email, password: 'password' }

    expect(session[:account_id]).to be_nil
    expect(response.body).to match(/closed|invalid|error/i)
  end
end
