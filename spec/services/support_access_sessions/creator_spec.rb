# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SupportAccessSessions::Creator do
  let(:account) { Account.create!(email: 'atomic-support-admin@example.test', status: :verified) }
  let(:household) { create(:household) }
  let(:support_session) do
    SupportAccessSession.new(
      platform_admin: PlatformAdmin.create!(account: account),
      household: household,
      reason: 'Investigate hosted support race',
      mfa_verified_at: Time.current
    )
  end

  it 'creates only after locking and authorizing the current household row' do
    expect do
      described_class.call(support_session: support_session, authorize: method(:authorize_support_session))
    end.to change(SupportAccessSession, :count).by(1)
  end

  it 'rejects stale authorization when a lifecycle transition wins the household lock' do
    Household.find(household.id).update!(lifecycle_state: :held)
    count = SupportAccessSession.count
    error = capture_error do
      described_class.call(support_session: support_session, authorize: method(:authorize_support_session))
    end

    expect([error.class, SupportAccessSession.count, support_session.household.lifecycle_state])
      .to eq([Pundit::NotAuthorizedError, count, 'held'])
  end

  def authorize_support_session(record)
    context = AuthorizationContext.new(account: account, household: nil, membership: nil)
    raise Pundit::NotAuthorizedError unless SupportAccessSessionPolicy.new(context, record).create?
  end

  def capture_error
    yield
  rescue StandardError => e
    e
  end
end
