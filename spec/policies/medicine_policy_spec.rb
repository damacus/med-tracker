# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe MedicinePolicy, type: :policy do
  fixtures :all

  subject(:policy) { described_class.new(current_user, medicine) }

  let(:medicine) { medicines(:paracetamol) }

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

    it 'permits most actions except destroy' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be true
        expect(policy.new?).to be true
        expect(policy.update?).to be true
        expect(policy.edit?).to be true
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

    it 'forbids all actions' do
      aggregate_failures do
        expect(policy.index?).to be false
        expect(policy.show?).to be false
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
      end
    end
  end

  describe 'for parent' do
    let(:current_user) { users(:parent) }

    it 'forbids all actions' do
      expect(policy.index?).to be false
      expect(policy.show?).to be false
      expect(policy.create?).to be false
      expect(policy.destroy?).to be false
    end
  end

  describe 'for patient' do
    let(:current_user) { users(:adult_patient) }

    it 'forbids all actions' do
      expect(policy.index?).to be false
      expect(policy.show?).to be false
      expect(policy.create?).to be false
      expect(policy.destroy?).to be false
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
    subject(:scope) { described_class::Scope.new(current_user, Medicine.all).resolve }

    context 'when user is an administrator' do
      let(:current_user) { users(:admin) }

      it 'returns all medicines' do
        expect(scope).to match_array(Medicine.all)
      end
    end

    context 'when user is a doctor' do
      let(:current_user) { users(:doctor) }

      it 'returns all medicines' do
        expect(scope).to match_array(Medicine.all)
      end
    end

    context 'when user is a nurse' do
      let(:current_user) { users(:nurse) }

      it 'returns all medicines' do
        expect(scope).to match_array(Medicine.all)
      end
    end

    context 'when user is a carer' do
      let(:current_user) { users(:carer) }

      it 'returns no medicines' do
        expect(scope).to be_empty
      end
    end

    context 'when user is a parent' do
      let(:current_user) { users(:parent) }

      it 'returns no medicines' do
        expect(scope).to be_empty
      end
    end

    context 'when user is nil' do
      let(:current_user) { nil }

      it 'returns no medicines' do
        expect(scope).to be_empty
      end
    end
  end
end
