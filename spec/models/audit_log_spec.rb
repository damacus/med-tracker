# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLog, type: :model do
  describe 'associations' do
    it { is_expected.to belong_to(:user).optional }
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:action) }
    it { is_expected.to validate_presence_of(:auditable_type) }
    it { is_expected.to validate_presence_of(:auditable_id) }
  end

  describe 'scopes' do
    let(:person) do
      Person.create!(
        name: 'Test Person',
        date_of_birth: 20.years.ago
      )
    end
    let(:user) do
      User.create!(
        email_address: "scope_test_#{SecureRandom.hex(4)}@example.com",
        password: 'password123',
        person: person
      )
    end
    let!(:old_log) do
      AuditLog.create!(
        action: 'create',
        auditable_type: 'Test',
        auditable_id: 1,
        created_at: 2.days.ago
      )
    end
    let!(:new_log) do
      AuditLog.create!(
        action: 'create',
        auditable_type: 'Test',
        auditable_id: 2,
        created_at: 1.day.ago
      )
    end

    describe '.recent' do
      it 'orders logs by created_at descending' do
        expect(AuditLog.recent.first).to eq(new_log)
        expect(AuditLog.recent.last).to eq(old_log)
      end
    end

    describe '.by_action' do
      let!(:create_log) do
        AuditLog.create!(
          action: 'create',
          auditable_type: 'User',
          auditable_id: user.id
        )
      end
      let!(:update_log) do
        AuditLog.create!(
          action: 'update',
          auditable_type: 'User',
          auditable_id: user.id
        )
      end

      it 'filters logs by action' do
        expect(AuditLog.by_action('create')).to include(create_log)
        expect(AuditLog.by_action('create')).not_to include(update_log)
      end
    end

    describe '.by_user' do
      let(:other_person) do
        Person.create!(
          name: 'Other Person',
          date_of_birth: 25.years.ago
        )
      end
      let(:other_user) do
        User.create!(
          email_address: "scope_other_#{SecureRandom.hex(4)}@example.com",
          password: 'password123',
          person: other_person
        )
      end
      let!(:user_log) do
        AuditLog.create!(
          user: user,
          action: 'create',
          auditable_type: 'User',
          auditable_id: user.id
        )
      end
      let!(:other_log) do
        AuditLog.create!(
          user: other_user,
          action: 'create',
          auditable_type: 'User',
          auditable_id: other_user.id
        )
      end

      it 'filters logs by user' do
        expect(AuditLog.by_user(user)).to include(user_log)
        expect(AuditLog.by_user(user)).not_to include(other_log)
      end
    end
  end

  describe '#action_description' do
    it 'returns human-readable description for create' do
      log = AuditLog.new(
        action: 'create',
        auditable_type: 'User',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Created User')
    end

    it 'returns human-readable description for update' do
      log = AuditLog.new(
        action: 'update',
        auditable_type: 'Person',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Updated Person')
    end

    it 'returns human-readable description for destroy' do
      log = AuditLog.new(
        action: 'destroy',
        auditable_type: 'CarerRelationship',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Deleted CarerRelationship')
    end

    it 'returns human-readable description for take_medicine' do
      log = AuditLog.new(
        action: 'take_medicine',
        auditable_type: 'MedicationTake',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Took medication')
    end
  end

  describe '#actor_name' do
    it 'returns user name when user is present' do
      person = Person.create!(
        name: 'Test Person',
        date_of_birth: 20.years.ago
      )
      user = User.create!(
        email_address: "actor_test_#{SecureRandom.hex(4)}@example.com",
        password: 'password123',
        person: person
      )
      log = AuditLog.new(
        user: user,
        action: 'create',
        auditable_type: 'User',
        auditable_id: user.id
      )
      expect(log.actor_name).to eq(user.name)
    end

    it 'returns "System" when user is nil' do
      log = AuditLog.new(
        user: nil,
        action: 'create',
        auditable_type: 'User',
        auditable_id: 1
      )
      expect(log.actor_name).to eq('System')
    end
  end
end
