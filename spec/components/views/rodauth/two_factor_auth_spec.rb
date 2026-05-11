# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::TwoFactorAuth, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles
  it 'renders the two-factor auth selection screen' do
    rodauth = double('Rodauth', two_factor_auth_links: [[10, '/otp-auth', 'Authenticator app']])

    allow(controller).to receive(:rodauth).and_return(rodauth)

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Additional authentication required')
    expect(rendered.text).to include('Authenticator app')
    expect(rendered.to_html).to include('min-h-screen')
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
