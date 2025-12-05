# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApplicationPolicy do
  fixtures :people

  let(:policy_class) do
    Class.new(ApplicationPolicy) do
      public :record_person_id
    end
  end

  let(:user) { nil }

  def policy(record)
    policy_class.new(user, record)
  end

  describe '#record_person_id' do
    it 'returns nil for nil record' do
      expect(policy(nil).record_person_id).to be_nil
    end

    it 'returns nil for Class records' do
      expect(policy(Person).record_person_id).to be_nil
    end

    it 'returns id when record is a Person' do
      person = people(:john)
      expect(policy(person).record_person_id).to eq(person.id)
    end

    it 'returns prescription.person_id when present' do
      prescription = instance_double('Prescription', person_id: 123)
      record = instance_double('RecordWithPrescription', prescription:)

      expect(policy(record).record_person_id).to eq(123)
    end

    it 'returns person.id when record has a person association' do
      person = people(:john)
      record = instance_double('RecordWithPerson', person:)

      expect(policy(record).record_person_id).to eq(person.id)
    end

    it 'returns person_id attribute when available' do
      record = instance_double('RecordWithPersonId', person_id: 456, person: nil)

      expect(policy(record).record_person_id).to eq(456)
    end
  end
end
