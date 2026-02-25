# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe PersonPolicy do
  fixtures :all

  subject(:policy) { described_class.new(current_user, person) }

  let(:person) { people(:john) }

  context 'when user is an administrator' do
    let(:current_user) { users(:admin) }

    it 'permits viewing and management actions except create' do
      %i[index show update edit destroy].each do |action|
        expect(policy.public_send("#{action}?")).to be true
      end
    end

    it 'forbids creating people' do
      expect(policy.create?).to be false
      expect(policy.new?).to be false
    end
  end

  context 'when user is a doctor' do
    let(:current_user) { users(:doctor) }

    it 'permits viewing but not management actions' do
      %i[index show].each { |action| expect(policy.public_send("#{action}?")).to be true }
      %i[create new update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end
  end

  context 'when user is a nurse' do
    let(:current_user) { users(:nurse) }

    it 'permits viewing but not management actions' do
      %i[index show].each { |action| expect(policy.public_send("#{action}?")).to be true }
      %i[create new update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end
  end

  context 'when user is a parent/carer' do
    let(:current_user) { users(:jane) }

    context 'with their own record' do
      let(:person) { current_user.person }

      it 'permits viewing only' do
        expect(policy.show?).to be true
        %i[update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
      end
    end

    context 'with an assigned patient' do
      let(:person) { people(:child_patient) }

      it 'permits viewing only' do
        expect(policy.show?).to be true
        %i[update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
      end
    end

    context 'with an unrelated person' do
      let(:person) { people(:john) }

      it 'forbids viewing' do
        expect(policy.show?).to be false
      end
    end

    it 'allows accessing index (but scope limits to assigned people)' do
      expect(policy.index?).to be true
    end

    context 'when creating a minor' do
      let(:person) { Person.new(person_type: :minor, date_of_birth: 5.years.ago) }

      it { expect(policy.create?).to be true }
    end

    context 'when creating a dependent adult' do
      let(:person) { Person.new(person_type: :dependent_adult, date_of_birth: 70.years.ago) }

      it { expect(policy.create?).to be true }
    end

    context 'when creating an adult' do
      let(:person) { Person.new(person_type: :adult, date_of_birth: 30.years.ago) }

      it { expect(policy.create?).to be false }
    end

    context 'when creating a new person (no type yet)' do
      let(:person) { Person.new }

      it { expect(policy.new?).to be true }
    end
  end

  context 'when user is a carer' do
    let(:current_user) { users(:carer) }

    context 'when creating a minor' do
      let(:person) { Person.new(person_type: :minor, date_of_birth: 5.years.ago) }

      it { expect(policy.create?).to be true }
    end

    context 'when creating an adult' do
      let(:person) { Person.new(person_type: :adult, date_of_birth: 30.years.ago) }

      it { expect(policy.create?).to be false }
    end
  end

  context 'when user is nil' do
    let(:current_user) { nil }

    it 'forbids all actions' do
      %i[index show create new update edit destroy].each do |action|
        expect(policy.public_send("#{action}?")).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { described_class::Scope.new(current_user, Person.all).resolve }

    context 'when user is an administrator' do
      let(:current_user) { users(:admin) }

      it 'returns all people' do
        expect(scope).to match_array(Person.all)
      end
    end

    context 'when user is a doctor' do
      let(:current_user) { users(:doctor) }

      it 'returns all people' do
        expect(scope).to match_array(Person.all)
      end
    end

    context 'when user is a nurse' do
      let(:current_user) { users(:nurse) }

      it 'returns all people' do
        expect(scope).to match_array(Person.all)
      end
    end

    context 'when user is a parent/carer' do
      let(:current_user) { users(:jane) }

      it 'returns only their person and assigned patients' do
        expected_people = [current_user.person, people(:child_patient)]
        expect(scope).to match_array(expected_people)
      end
    end

    context 'when user is nil' do
      let(:current_user) { nil }

      it 'returns no records' do
        expect(scope).to be_empty
      end
    end
  end
end
