# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Context do
  fixtures :accounts, :people, :users

  let(:account) { accounts(:admin) }
  let(:user) { users(:admin) }
  let(:household) { user.person.household || create(:household) }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: account) do |record|
      record.person = user.person
      record.role = :owner
      record.status = :active
    end
  end
  let(:request) do
    instance_double(
      ActionDispatch::Request,
      request_id: 'req-audit-context',
      remote_ip: '192.0.2.10',
      session: Struct.new(:id).new('raw-browser-session-id')
    )
  end

  after { described_class.clear! }

  it 'captures actor, role, request, and opaque web-session context', :aggregate_failures do
    described_class.start!(request:, account:, user:, membership:, credential: :web)

    expect(described_class.current).to include(
      actor_account_id: account.id,
      actor_user_id: user.id,
      actor_membership_id: membership.id,
      active_role: 'owner',
      permissions_version: membership.permissions_version,
      authentication_method: 'web',
      request_id: 'req-audit-context',
      ip: '192.0.2.10'
    )
    expect(described_class.current.fetch(:session_reference)).to start_with('web:')
    expect(described_class.current.to_json).not_to include('raw-browser-session-id')
  end

  it 'captures the successful policy and query in PaperTrail metadata', :aggregate_failures do
    described_class.start!(request:, account:, user:, membership:, credential: :web)
    described_class.authorized!(policy_class: MedicationPolicy, query: :update?)

    expect(described_class.current).to include(
      policy_class: 'MedicationPolicy',
      policy_query: 'update?'
    )
    expect(PaperTrail.request.controller_info.fetch(:audit_context)).to include(
      policy_class: 'MedicationPolicy',
      policy_query: 'update?'
    )
  end

  it 'clears request-local context and PaperTrail metadata' do
    described_class.start!(request:, account:, user:, membership:, credential: :web)

    described_class.clear!

    expect(Current.audit_context).to be_nil
    expect(PaperTrail.request.controller_info).to eq({})
    expect(PaperTrail.request.whodunnit).to be_nil
  end
end
