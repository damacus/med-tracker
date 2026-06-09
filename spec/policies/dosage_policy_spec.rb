# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DosagePolicy, type: :policy do
  fixtures :all

  subject(:policy) { described_class.new(user, dosage) }

  let(:dosage) { dosages(:paracetamol_adult) }

  context 'when the user is a doctor (can update medications, cannot destroy them)' do
    let(:user) { users(:doctor) }

    it 'mirrors MedicationPolicy#update? for write actions (destroy? via update?)' do
      aggregate_failures do
        expect(policy.show?).to be(true)
        expect(policy.create?).to be(true)
        expect(policy.new?).to be(true)
        expect(policy.update?).to be(true)
        expect(policy.edit?).to be(true)
        expect(policy.destroy?).to be(true)
      end
    end
  end

  context 'when the user is a nurse (read-only on medications)' do
    let(:user) { users(:nurse) }

    it 'denies create/update/edit/destroy but allows show' do
      aggregate_failures do
        expect(policy.create?).to be(false)
        expect(policy.new?).to be(false)
        expect(policy.update?).to be(false)
        expect(policy.edit?).to be(false)
        expect(policy.destroy?).to be(false)
        expect(policy.show?).to be(true)
      end
    end
  end

  context 'without a user' do
    let(:user) { nil }

    it 'denies all actions' do
      aggregate_failures do
        expect(policy.show?).to be(false)
        expect(policy.create?).to be(false)
        expect(policy.update?).to be(false)
        expect(policy.destroy?).to be(false)
      end
    end
  end
end
