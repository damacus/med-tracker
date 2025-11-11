# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLog do
  fixtures :users, :people, :audit_logs

  describe 'associations' do
    it { is_expected.to belong_to(:user).optional }
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:action) }
    it { is_expected.to validate_presence_of(:auditable_type) }
    it { is_expected.to validate_presence_of(:auditable_id) }
  end

  describe 'scopes' do
    describe '.recent' do
      it 'orders logs by created_at descending' do
        logs = described_class.recent.limit(2)
        expect(logs.first.created_at).to be >= logs.last.created_at
      end
    end

    describe '.by_action' do
      it 'filters logs by action' do
        create_logs = described_class.by_action('create')
        expect(create_logs).to all(have_attributes(action: 'create'))
      end
    end

    describe '.by_user' do
      let(:user) { users(:admin) }

      it 'filters logs by user' do
        user_logs = described_class.by_user(user)
        expect(user_logs).to all(have_attributes(user: user))
      end
    end
  end

  describe '#action_description' do
    it 'returns human-readable description for create' do
      log = audit_logs(:user_created)
      expect(log.action_description).to eq('Created User')
    end

    it 'returns human-readable description for update' do
      log = audit_logs(:person_updated)
      expect(log.action_description).to eq('Updated Person')
    end

    it 'returns human-readable description for destroy' do
      log = described_class.new(
        action: 'destroy',
        auditable_type: 'CarerRelationship',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Deleted CarerRelationship')
    end

    it 'returns human-readable description for take_medicine' do
      log = described_class.new(
        action: 'take_medicine',
        auditable_type: 'MedicationTake',
        auditable_id: 1
      )
      expect(log.action_description).to eq('Took medication')
    end
  end

  describe '#actor_name' do
    it 'returns user name when user is present' do
      log = audit_logs(:user_created)
      expect(log.actor_name).to eq(log.user.name)
    end

    it 'returns "System" when user is nil' do
      log = described_class.new(
        user: nil,
        action: 'create',
        auditable_type: 'User',
        auditable_id: 1
      )
      expect(log.actor_name).to eq('System')
    end
  end
end
