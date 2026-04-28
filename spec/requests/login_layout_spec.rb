# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Login layout' do
  it 'renders without the global mobile navigation chrome' do
    get login_path

    expect(response.body).not_to include('class="nav"')
    expect(response.body).not_to include('nav__brand-link')
  end

  it 'redirects unauthenticated users to login without routine login-required flash' do
    get dashboard_path

    expect(response).to redirect_to(login_path)
    expect(flash[:alert]).to be_blank
    expect(flash[:notice]).to be_blank

    follow_redirect!

    expect(response.body).not_to include('Please login to continue')
    expect(response.body.scan('role="alert"').count).to eq(0)
  end
end
