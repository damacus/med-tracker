# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLogPolicy do
  subject(:policy) { described_class.new(user, AuditLog) }

  describe 'for administrator' do
    let(:person) do
      Person.create!(name: 'Admin Person', date_of_birth: 30.years.ago)
    end
    let(:user) do
      User.create!(
        email_address: "admin_#{SecureRandom.hex(4)}@example.com",
        password: 'password123',
        person: person,
        role: :administrator
      )
    end

    it { is_expected.to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  context 'when user is not an administrator' do
    let(:person) do
      Person.create!(name: 'Regular User', date_of_birth: 30.years.ago)
    end
    let(:user) do
      User.create!(
        email_address: "user_#{SecureRandom.hex(4)}@example.com",
        password: 'password123',
        person: person,
        role: :parent
      )
    end

    it { is_expected.not_to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  describe 'for nil user' do
    let(:user) { nil }

    it { is_expected.not_to permit_action(:index) }
    it { is_expected.not_to permit_action(:create) }
    it { is_expected.not_to permit_action(:update) }
    it { is_expected.not_to permit_action(:destroy) }
  end

  describe 'Scope' do
    let(:admin_person) do
      Person.create!(name: 'Admin', date_of_birth: 30.years.ago)
    end
    let(:admin_user) do
      User.create!(
        email_address: "admin_scope_#{SecureRandom.hex(4)}@example.com",
        password: 'password123',
        person: admin_person,
        role: :administrator
      )
    end

    let!(:audit_log1) do
      AuditLog.create!(
        action: 'create',
        auditable_type: 'User',
        auditable_id: admin_user.id
      )
    end
    let!(:audit_log2) do
      AuditLog.create!(
        action: 'update',
        auditable_type: 'Person',
        auditable_id: admin_person.id
      )
    end

    context 'when user is an administrator' do
      it 'returns all audit logs' do
        resolved_scope = Pundit.policy_scope(admin_user, AuditLog)
        expect(resolved_scope).to include(audit_log1, audit_log2)
        expect(resolved_scope.count).to be >= 2
      end
    end

    context 'when user is not an administrator' do
      let(:regular_person) do
        Person.create!(name: 'Regular', date_of_birth: 25.years.ago)
      end
      let(:regular_user) do
        User.create!(
          email_address: "regular_#{SecureRandom.hex(4)}@example.com",
          password: 'password123',
          person: regular_person,
          role: :parent
        )
      end

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
