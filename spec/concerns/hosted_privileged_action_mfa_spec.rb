# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HostedPrivilegedActionMfa do
  subject(:host) { host_class.new(session) }

  let(:session) { {} }
  let(:host_class) do
    Class.new do
      include HostedPrivilegedActionMfa

      attr_accessor :default_household_for_urls
      attr_reader :session

      def initialize(session)
        @session = session
      end

      def profile_path
        '/profile'
      end

      def rodauth
        Struct.new(:otp_setup_path).new('/otp-setup')
      end
    end
  end

  around do |example|
    Current.reset
    example.run
  ensure
    Current.reset
  end

  it 'returns nil when no privileged MFA timestamp is stored in the session' do
    expect(host.send(:privileged_action_mfa_verified_at)).to be_nil
  end

  it 'reads a string-keyed privileged MFA timestamp from the session' do
    timestamp = Time.current.to_i
    session['privileged_action_mfa_verified_at'] = timestamp

    expect(host.send(:privileged_action_mfa_verified_at)).to eq(Time.zone.at(timestamp))
  end

  it 'uses the profile setup path when household URL context is available' do
    host.default_household_for_urls = Household.new

    expect(host.send(:privileged_action_mfa_setup_path)).to eq('/profile')
  end

  it 'falls back to the Rodauth OTP setup path without household URL context' do
    expect(host.send(:privileged_action_mfa_setup_path)).to eq('/otp-setup')
  end
end
