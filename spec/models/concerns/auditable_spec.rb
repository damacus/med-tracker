# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Auditable, type: :model do
  fixtures :users, :people, :sessions

  let(:user) { users(:admin) }
  let(:session) { sessions(:admin_session) }

  before do
    Current.session = session
  end

  after do
    Current.reset
  end

  describe 'audit logging on create' do
    it 'creates an audit log when a person is created' do
      person = Person.new(
        name: 'New Test Person',
        date_of_birth: 20.years.ago
      )

      expect {
        person.save!
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('create')
      expect(audit_log.auditable_type).to eq('Person')
      expect(audit_log.auditable_id).to eq(person.id)
      expect(audit_log.user).to eq(user)
    end
  end

  describe 'audit logging on update' do
    let(:person) { people(:john) }

    before do
      # Clear any audit logs from fixtures
      AuditLog.delete_all
    end

    it 'creates an audit log when a person is updated' do
      expect {
        person.update!(name: 'Updated Name')
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('update')
      expect(audit_log.auditable_type).to eq('Person')
      expect(audit_log.auditable_id).to eq(person.id)
      expect(audit_log.change_data['name']).to eq(['John Doe', 'Updated Name'])
    end

    it 'does not create audit log for insignificant changes' do
      # Touch only updates updated_at
      expect {
        person.touch
      }.not_to change(AuditLog, :count)
    end
  end

  describe 'audit logging on destroy' do
    let!(:person) do
      # Create a new person not from fixtures to avoid cascade deletions
      Person.create!(
        name: 'Person to Delete',
        date_of_birth: 30.years.ago
      )
    end

    before do
      # Clear any audit logs from creation
      AuditLog.delete_all
    end

    it 'creates an audit log when a person is destroyed' do
      person_id = person.id
      expect {
        person.destroy!
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('destroy')
      expect(audit_log.auditable_type).to eq('Person')
      expect(audit_log.auditable_id).to eq(person_id)
    end
  end

  describe 'audit logging for users' do
    it 'creates audit log when user is created' do
      new_person = Person.create!(
        name: 'New User Person',
        date_of_birth: 25.years.ago
      )

      # Clear person creation audit log
      AuditLog.delete_all

      expect {
        User.create!(
          email_address: 'newuser@example.com',
          password: 'password123',
          person: new_person
        )
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('create')
      expect(audit_log.auditable_type).to eq('User')
    end
  end

  describe 'audit logging for carer relationships' do
    fixtures :carer_relationships

    let(:carer) { people(:bob) }
    let(:patient) { people(:child_patient) }

    it 'creates audit log when carer relationship is created' do
      # Clear audit logs
      AuditLog.delete_all

      expect {
        CarerRelationship.create!(
          carer: carer,
          patient: patient,
          relationship_type: 'parent'
        )
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('create')
      expect(audit_log.auditable_type).to eq('CarerRelationship')
    end
  end

  describe 'audit logging for medication takes' do
    fixtures :medicines, :dosages, :prescriptions

    let(:prescription) { prescriptions(:john_paracetamol) }

    it 'creates audit log when medication is taken' do
      # Clear audit logs
      AuditLog.delete_all

      expect {
        MedicationTake.create!(
          prescription: prescription,
          taken_at: Time.current,
          amount_ml: 5
        )
      }.to change(AuditLog, :count).by(1)

      audit_log = AuditLog.last
      expect(audit_log.action).to eq('create')
      expect(audit_log.auditable_type).to eq('MedicationTake')
    end
  end
end
