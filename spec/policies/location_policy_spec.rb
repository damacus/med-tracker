# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe LocationPolicy, type: :policy do
  fixtures :all

  subject(:policy) { described_class.new(current_user, location) }

  let(:location) { locations(:home) }

  describe 'for administrator' do
    let(:current_user) { users(:admin) }

    it 'permits all actions' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be true
        expect(policy.new?).to be true
        expect(policy.update?).to be true
        expect(policy.edit?).to be true
        expect(policy.destroy?).to be true
      end
    end
  end

  describe 'for doctor' do
    let(:current_user) { users(:doctor) }

    it 'permits viewing only' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
      end
    end
  end

  describe 'for nurse' do
    let(:current_user) { users(:nurse) }

    it 'permits viewing only' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
      end
    end
  end

  describe 'for carer' do
    let(:current_user) { users(:carer) }

    it 'permits viewing only' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.destroy?).to be false
      end
    end

    it 'denies viewing locations outside their household' do
      foreign_location = locations(:school)
      scoped_policy = described_class.new(current_user, foreign_location)
      expect(scoped_policy.show?).to be false
    end
  end

  describe 'for parent' do
    let(:current_user) { users(:parent) }

    it 'permits viewing only' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.destroy?).to be false
      end
    end

    it 'permits viewing a dependent adult patient location' do
      dependent_adult = Person.new(
        name: 'Dependent Adult',
        date_of_birth: 70.years.ago.to_date,
        person_type: :dependent_adult,
        has_capacity: false,
        primary_location: locations(:school)
      )
      dependent_adult.carer_relationships.build(carer: current_user.person, relationship_type: :parent, active: true)
      dependent_adult.save!

      expect(described_class.new(current_user, locations(:school)).show?).to be true
    end
  end

  describe 'for nil user' do
    let(:current_user) { nil }

    it 'forbids all actions' do
      %i[index show create new update edit destroy].each do |action|
        expect(policy.public_send("#{action}?")).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { described_class::Scope.new(current_user, Location.all).resolve }

    context 'when user is a carer' do
      let(:current_user) { users(:carer) }

      it 'returns only their household locations' do
        expect(scope).to contain_exactly(locations(:home))
      end
    end

    context 'when user is a parent with multiple locations' do
      let(:current_user) { users(:jane) }

      it 'returns the locations tied to their household' do
        expect(scope).to contain_exactly(locations(:home), locations(:school))
      end
    end

    context 'when user is a parent with a dependent adult patient' do
      let(:current_user) { users(:parent) }

      before do
        dependent_adult = Person.new(
          name: 'Dependent Adult',
          date_of_birth: 70.years.ago.to_date,
          person_type: :dependent_adult,
          has_capacity: false,
          primary_location: locations(:school)
        )
        dependent_adult.carer_relationships.build(carer: current_user.person, relationship_type: :parent, active: true)
        dependent_adult.save!
      end

      it 'returns the dependent adult patient locations' do
        expect(scope).to contain_exactly(locations(:home), locations(:school))
      end
    end

    context 'when user is nil' do
      let(:current_user) { nil }

      it 'returns no locations' do
        expect(scope).to be_empty
      end
    end
  end
end
