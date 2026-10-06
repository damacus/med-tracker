# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ExpireSupportAccessSessionsJob do
  it 'runs the idempotent support-session expiry processor' do
    allow(SupportAccessSessions::ExpiryProcessor).to receive(:call).and_return(2)

    expect(described_class.perform_now).to eq(2)
    expect(SupportAccessSessions::ExpiryProcessor).to have_received(:call).once
  end
end
