# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLogPolicy do
  subject(:policy) { described_class.new(user, AuditLog) }

  fixtures :users, :people

  context 'when user is an administrator' do
    let(:user) { users(:admin) }

    it { is_expected.to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  context 'when user is not an administrator' do
    let(:user) { users(:parent) }

    it { is_expected.not_to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  context 'when user is nil' do
    let(:user) { nil }

    it { is_expected.not_to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  describe 'Scope' do
    fixtures :users, :people, :audit_logs

    context 'when user is an administrator' do
      let(:admin_user) { users(:admin) }

      it 'returns all audit logs' do
        resolved_scope = Pundit.policy_scope(admin_user, AuditLog)
        expect(resolved_scope.count).to be >= 2
        expect(resolved_scope).to include(audit_logs(:user_created), audit_logs(:person_updated))
      end
    end

    context 'when user is not an administrator' do
      let(:regular_user) { users(:parent) }

      it 'returns no audit logs' do
        resolved_scope = Pundit.policy_scope(regular_user, AuditLog)
        expect(resolved_scope).to be_empty
      end
    end

    context 'when user is nil' do
      it 'returns no audit logs' do
        resolved_scope = Pundit.policy_scope(nil, AuditLog)
        expect(resolved_scope).to be_empty
      end
    end
  end
end
