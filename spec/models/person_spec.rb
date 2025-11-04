# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Person do
  subject(:person) do
    described_class.new(
      name: 'Jane Smith',
      email: 'jane.smith@example.com',
      date_of_birth: Date.new(2010, 6, 15)
    )
  end

  describe 'associations' do
    it { is_expected.to have_one(:user).inverse_of(:person).dependent(:destroy) }
    it { is_expected.to have_many(:prescriptions).dependent(:destroy) }
    it { is_expected.to have_many(:medicines).through(:prescriptions) }
    it { is_expected.to have_many(:carer_relationships).dependent(:destroy) }
    it { is_expected.to have_many(:carers).through(:carer_relationships) }
    it { is_expected.to have_many(:patient_relationships).dependent(:destroy) }
    it { is_expected.to have_many(:patients).through(:patient_relationships) }
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }
    it { is_expected.to validate_presence_of(:date_of_birth) }
    it { is_expected.to validate_uniqueness_of(:email).case_insensitive }

    it 'allows a blank email' do
      person.email = ''
      expect(person).to be_valid
    end

    it 'enforces case-insensitive uniqueness when email is present' do
      described_class.create!(
        name: 'Existing Person',
        email: 'duplicate@example.com',
        date_of_birth: Date.new(1990, 1, 1)
      )

      person.email = 'DUPLICATE@example.com'
      expect(person).not_to be_valid
      expect(person.errors[:email]).to include('has already been taken')
    end

    it 'allows a person without a user account' do
      expect(person).to be_valid
    end
  end

  describe 'person types' do
    it 'defaults to adult type' do
      new_person = described_class.create!(
        name: 'Default Person',
        date_of_birth: 20.years.ago
      )

      expect(new_person.adult?).to be true
    end

    it 'can be a minor' do
      minor = described_class.create!(
        name: 'Minor Person',
        date_of_birth: 10.years.ago,
        person_type: :minor
      )

      expect(minor.minor?).to be true
    end

    it 'can be a dependent adult' do
      dependent = described_class.create!(
        name: 'Dependent Adult',
        date_of_birth: 75.years.ago,
        person_type: :dependent_adult
      )

      expect(dependent.dependent_adult?).to be true
    end
  end

  describe '#age' do
    it 'calculates age correctly' do
      person_twenty_five = described_class.new(
        name: 'Test Person',
        date_of_birth: 25.years.ago
      )

      expect(person_twenty_five.age).to eq(25)
    end

    it 'handles birthdays correctly' do
      today = Date.new(2024, 6, 15)
      person_before_birthday = described_class.new(
        name: 'Test Person',
        date_of_birth: Date.new(2000, 6, 16)
      )

      expect(person_before_birthday.age(today)).to eq(23)
    end

    it 'returns nil when date_of_birth is nil' do
      person_no_dob = described_class.new(name: 'Test Person')
      expect(person_no_dob.age).to be_nil
    end
  end

  describe '#adult?' do
    it 'returns true for person 18 years or older' do
      adult = described_class.new(
        name: 'Adult Person',
        date_of_birth: 25.years.ago
      )

      expect(adult.adult?).to be true
    end

    it 'returns true for person exactly 18 years old' do
      adult_eighteen = described_class.new(
        name: 'Just Adult',
        date_of_birth: 18.years.ago
      )

      expect(adult_eighteen.adult?).to be true
    end

    it 'returns false for person under 18' do
      minor = described_class.new(
        name: 'Minor Person',
        date_of_birth: 10.years.ago
      )

      expect(minor.adult?).to be false
    end

    it 'returns false when age is nil' do
      person_no_dob = described_class.new(name: 'No DOB')
      expect(person_no_dob.adult?).to be false
    end

    it 'accepts custom age threshold' do
      person_twenty = described_class.new(
        name: 'Twenty Year Old',
        date_of_birth: 20.years.ago
      )

      expect(person_twenty.adult?(21)).to be false
      expect(person_twenty.adult?(20)).to be true
      expect(person_twenty.adult?(18)).to be true
    end
  end

  describe '#minor?' do
    it 'returns false for person 18 years or older' do
      adult = described_class.new(
        name: 'Adult Person',
        date_of_birth: 25.years.ago
      )

      expect(adult.minor?).to be false
    end

    it 'returns true for person under 18' do
      minor = described_class.new(
        name: 'Minor Person',
        date_of_birth: 10.years.ago
      )

      expect(minor.minor?).to be true
    end

    it 'returns false for person exactly 18 years old' do
      adult_eighteen = described_class.new(
        name: 'Just Adult',
        date_of_birth: 18.years.ago
      )

      expect(adult_eighteen.minor?).to be false
    end

    it 'accepts custom age threshold' do
      person_twenty = described_class.new(
        name: 'Twenty Year Old',
        date_of_birth: 20.years.ago
      )

      expect(person_twenty.minor?(21)).to be true
      expect(person_twenty.minor?(20)).to be false
      expect(person_twenty.minor?(18)).to be false
    end
  end

  describe 'capacity' do
    it 'has capacity by default' do
      new_person = described_class.create!(
        name: 'Capable Person',
        date_of_birth: 20.years.ago
      )

      expect(new_person.has_capacity).to be true
    end

    it 'can be marked as lacking capacity' do
      person_without_capacity = described_class.create!(
        name: 'Person Without Capacity',
        date_of_birth: 5.years.ago,
        has_capacity: false
      )

      expect(person_without_capacity.has_capacity).to be false
    end
  end

  describe 'carer relationships' do
    let(:patient) do
      described_class.create!(
        name: 'Child Patient',
        date_of_birth: 5.years.ago,
        person_type: :minor,
        has_capacity: false
      )
    end

    let(:carer) do
      described_class.create!(
        name: 'Parent Carer',
        date_of_birth: 35.years.ago,
        person_type: :adult
      )
    end

    it 'can have carers assigned' do
      patient.carer_relationships.create!(carer: carer, relationship_type: 'parent')

      expect(patient.carers).to include(carer)
      expect(carer.patients).to include(patient)
    end

    it 'can have multiple carers' do
      carer2 = described_class.create!(
        name: 'Second Carer',
        date_of_birth: 40.years.ago,
        person_type: :adult
      )

      patient.carer_relationships.create!(carer: carer, relationship_type: 'parent')
      patient.carer_relationships.create!(carer: carer2, relationship_type: 'guardian')

      expect(patient.carers.count).to eq(2)
    end

    it 'can specify relationship type' do
      relationship = patient.carer_relationships.create!(
        carer: carer,
        relationship_type: 'parent'
      )

      expect(relationship.relationship_type).to eq('parent')
    end
  end
end
