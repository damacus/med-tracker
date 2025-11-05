# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Auditable, type: :model do
  # Use Person as the test model since it includes Auditable
  let(:person) do
    Person.new(
      name: 'Test Person',
      date_of_birth: 20.years.ago
    )
  end

  let(:user_person) do
    Person.create!(
      name: 'Logged In User',
      date_of_birth: 30.years.ago
    )
  end

  let(:user) do
    User.create!(
      email_address: 'user@example.com',
      password: 'password123',
      person: user_person
    )
  end

  let(:session) do
    Session.create!(
      user: user,
      ip_address: '127.0.0.1',
      user_agent: 'Test Agent'
    )
  end

  before do
    Current.session = session
  end

  after do
    Current.reset
  end

  describe 'audit logging on create' do
    it 'creates an audit log when a person is created' do
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
    before do
      person.save!
      # Clear the create audit log from the count
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
      expect(audit_log.change_data['name']).to eq(['Test Person', 'Updated Name'])
    end

    it 'does not create audit log for insignificant changes' do
      # Touch only updates updated_at
      expect {
        person.touch
      }.not_to change(AuditLog, :count)
    end
  end

  describe 'audit logging on destroy' do
    before do
      person.save!
      # Clear the create audit log from the count
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
    let(:carer) do
      Person.create!(
        name: 'Carer',
        date_of_birth: 40.years.ago
      )
    end

    let(:patient) do
      Person.create!(
        name: 'Patient',
        date_of_birth: 10.years.ago,
        person_type: :minor
      )
    end

    it 'creates audit log when carer relationship is created' do
      # Clear audit logs from creating carer and patient
      carer
      patient
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
    let(:medicine) do
      Medicine.create!(
        name: 'Test Medicine',
        current_supply: 100,
        stock: 100
      )
    end

    let(:dosage) do
      Dosage.create!(
        medicine: medicine,
        amount: 5,
        unit: 'ml',
        frequency: 'twice daily'
      )
    end

    let(:patient) do
      Person.create!(
        name: 'Patient',
        date_of_birth: 10.years.ago
      )
    end

    let(:prescription) do
      Prescription.create!(
        person: patient,
        medicine: medicine,
        dosage: dosage,
        start_date: Date.today,
        end_date: Date.today + 30.days
      )
    end

    it 'creates audit log when medication is taken' do
      # Pre-create all dependencies
      medicine
      dosage
      patient
      prescription
      # Clear audit logs from creating dependencies
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
