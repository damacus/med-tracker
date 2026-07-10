# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Login layout' do
  it 'uses one non-empty CSP nonce across the anonymous response' do
    get login_path

    document = response.parsed_body
    nonce = document.at_css('meta[name="csp-nonce"]')['content']
    activation_script = document.css('script[nonce]').find do |script|
      script.text.include?('applyLoginIllustrations')
    end

    expect(nonce).to be_present
    expect(response.headers.fetch('Content-Security-Policy')).to include("'nonce-#{nonce}'")
    expect(activation_script['nonce']).to eq(nonce)
  end

  it 'generates a new CSP nonce for each anonymous request' do
    get login_path
    first_nonce = response.parsed_body.at_css('meta[name="csp-nonce"]')['content']

    get login_path
    second_nonce = response.parsed_body.at_css('meta[name="csp-nonce"]')['content']

    expect(first_nonce).to be_present
    expect(second_nonce).to be_present
    expect(second_nonce).not_to eq(first_nonce)
  end

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
