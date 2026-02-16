# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe CarerRelationshipPolicy do
  fixtures :accounts, :people, :users, :carer_relationships, :person_medicines, :medication_takes

  subject(:policy) { described_class.new(current_user, relationship) }

  let(:relationship) { carer_relationships(:jane_cares_for_child) }

  context 'when user is an administrator' do
    let(:current_user) { users(:admin) }

    it 'permits all actions' do
      %i[index show create new update edit destroy].each do |action|
        expect(policy.public_send("#{action}?")).to be true
      end
    end
  end

  context 'when user is a doctor' do
    let(:current_user) { users(:doctor) }

    it 'permits viewing but not management' do
      %i[index show].each { |action| expect(policy.public_send("#{action}?")).to be true }
      %i[create new update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end
  end

  context 'when user is a nurse' do
    let(:current_user) { users(:nurse) }

    it 'permits viewing but not management' do
      %i[index show].each { |action| expect(policy.public_send("#{action}?")).to be true }
      %i[create new update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end
  end

  context 'when user is a carer' do
    let(:current_user) { users(:jane) }

    context 'with their own carer relationship' do
      let(:relationship) { carer_relationships(:jane_cares_for_child) }

      it 'permits viewing only' do
        expect(policy.show?).to be true
        %i[update edit destroy].each { |action| expect(policy.public_send("#{action}?")).to be false }
      end
    end

    context 'with another carer relationship' do
      let(:relationship) { carer_relationships(:nurse_cares_for_john) }

      it 'forbids viewing' do
        expect(policy.show?).to be false
      end
    end

    it 'forbids listing all relationships' do
      expect(policy.index?).to be false
    end

    it 'forbids creating relationships' do
      %i[create new].each { |action| expect(policy.public_send("#{action}?")).to be false }
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
    subject(:scope) { described_class::Scope.new(current_user, CarerRelationship.all).resolve }

    context 'when user is an administrator' do
      let(:current_user) { users(:admin) }

      it 'returns all relationships' do
        expect(scope).to match_array(CarerRelationship.all)
      end
    end

    context 'when user is a doctor' do
      let(:current_user) { users(:doctor) }

      it 'returns all relationships' do
        expect(scope).to match_array(CarerRelationship.all)
      end
    end

    context 'when user is a nurse' do
      let(:current_user) { users(:nurse) }

      it 'returns all relationships' do
        expect(scope).to match_array(CarerRelationship.all)
      end
    end

    context 'when user is a carer' do
      let(:current_user) { users(:jane) }

      it 'returns only their carer relationships' do
        expected = CarerRelationship.where(carer: current_user.person)
        expect(scope).to match_array(expected)
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
