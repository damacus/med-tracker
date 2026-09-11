# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuthenticationLifetime do
  subject(:auth) { RodauthApp.rodauth.allocate }

  before do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('SESSION_INACTIVITY_TIMEOUT_DAYS', '30').and_return('30')
    allow(ENV).to receive(:fetch).with('SESSION_MAX_AGE_DAYS', '0').and_return('0')
  end

  it 'keeps an active login valid without a forced absolute deadline by default' do
    expect(auth.session_inactivity_deadline).to eq(30.days.to_i)
    expect(auth.session_lifetime_deadline).to be_nil
  end

  it 'uses the environment-configured inactivity timeout for remembered login' do
    allow(ENV).to receive(:fetch).with('SESSION_INACTIVITY_TIMEOUT_DAYS', '30').and_return('7')

    expect(auth.session_inactivity_deadline).to eq(7.days.to_i)
    expect(auth.remember_deadline_interval).to eq(days: 7)
    expect(auth.remember_period).to eq(days: 7)
  end

  it 'allows an installation to configure a maximum login age' do
    allow(ENV).to receive(:fetch).with('SESSION_MAX_AGE_DAYS', '0').and_return('90')

    expect(auth.session_lifetime_deadline).to eq(90.days.to_i)
  end

  it 'rejects invalid configuration rather than silently disabling expiry' do
    %w[invalid -1 0].each do |value|
      allow(ENV).to receive(:fetch).with('SESSION_INACTIVITY_TIMEOUT_DAYS', '30').and_return(value)

      expect { auth.session_inactivity_deadline }.to raise_error(ArgumentError)
    end
  end
end
