# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::TwoFactorAuth, type: :component do
  it 'renders the two-factor auth selection screen' do
    rodauth = Struct.new(:two_factor_auth_links).new([[10, '/otp-auth', 'Authenticator app']])

    allow(controller).to receive(:rodauth).and_return(rodauth)

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Additional authentication required')
    expect(rendered.text).to include('Authenticator app')
    expect(rendered.to_html).to include('min-h-screen')
  end
end
