# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPolicy, type: :policy do
  fixtures :all
  subject(:policy) { described_class.new(current_user, medication) }

  let(:medication) { medications(:ibuprofen) }

  describe 'for admin' do
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
        expect(policy.finder?).to be true
      end
    end
  end

  describe 'for doctor' do
    let(:current_user) { users(:doctor) }

    it 'permits most actions except deletion' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be true
        expect(policy.new?).to be true
        expect(policy.update?).to be true
        expect(policy.edit?).to be true
        expect(policy.destroy?).to be false
        expect(policy.finder?).to be true
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
        expect(policy.finder?).to be false
      end
    end
  end

  describe 'for carer' do
    let(:current_user) { users(:carer) }

    it 'permits viewing but forbids write actions' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.finder?).to be false
      end
    end
  end

  describe 'for parent' do
    let(:current_user) { users(:parent) }

    it 'permits viewing and creating but forbids edit/removal actions' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be true
        expect(policy.new?).to be true
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.finder?).to be true
      end
    end
  end

  describe 'for patient (carer role managing own care)' do
    let(:current_user) { users(:adult_patient) }

    it 'permits viewing but forbids write actions' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.finder?).to be false
      end
    end
  end

  describe 'for nil user' do
    let(:current_user) { nil }

    it 'forbids all actions' do
      aggregate_failures do
        expect(policy.index?).to be false
        expect(policy.show?).to be false
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.finder?).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { MedicationPolicy::Scope.new(current_user, Medication.all).resolve }

    describe 'for admin' do
      let(:current_user) { users(:admin) }

      it 'returns all medications' do
        expect(scope).to include(medications(:ibuprofen))
      end
    end

    describe 'for doctor' do
      let(:current_user) { users(:doctor) }

      it 'returns all medications' do
        expect(scope).to include(medications(:ibuprofen))
      end
    end

    describe 'for nurse' do
      let(:current_user) { users(:nurse) }

      it 'returns all medications' do
        expect(scope).to include(medications(:ibuprofen))
      end
    end

    describe 'for carer' do
      let(:current_user) { users(:carer) }

      it 'returns medications at their locations' do
        # Carer has relationship with Adult Patient and Child Patient at Grandma's House
        # Medications ibuprofen and paracetamol are at Grandma's House
        expect(scope).to include(medications(:ibuprofen))
      end
    end

    describe 'for parent' do
      let(:current_user) { users(:parent) }

      it 'returns medications at their locations' do
        # Parent is at Home, Child Patient is also at Home
        expect(scope).to include(medications(:paracetamol))
      end
    end

    describe 'for unauthenticated user' do
      let(:current_user) { nil }

      it 'returns no medications' do
        expect(scope).to be_empty
      end
    end
  end
end
