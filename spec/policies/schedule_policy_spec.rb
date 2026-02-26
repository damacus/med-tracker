# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe SchedulePolicy, type: :policy do
  fixtures :all

  subject(:policy) { described_class.new(current_user, schedule) }

  let(:schedule) { schedules(:adult_patient_schedule) }

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
        expect(policy.take_medication?).to be true
      end
    end
  end

  describe 'for doctor' do
    let(:current_user) { users(:doctor) }

    it 'permits all actions' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be true
        expect(policy.new?).to be true
        expect(policy.update?).to be true
        expect(policy.edit?).to be true
        expect(policy.destroy?).to be true
        expect(policy.take_medication?).to be true
      end
    end
  end

  describe 'for nurse' do
    let(:current_user) { users(:nurse) }

    it 'permits viewing and administering only' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.take_medication?).to be true
      end
    end
  end

  describe 'for carer with assigned patient' do
    let(:current_user) { users(:carer) }
    let(:schedule) { schedules(:patient_schedule) }

    it 'permits viewing and administering' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.take_medication?).to be true
      end
    end
  end

  describe 'for carer without assigned patient' do
    let(:current_user) { users(:carer) }
    let(:schedule) { schedules(:adult_patient_schedule) }

    it 'forbids all actions' do
      expect(policy.index?).to be false
      expect(policy.show?).to be false
      expect(policy.take_medication?).to be false
    end
  end

  describe 'for parent with child' do
    let(:current_user) { users(:parent) }
    let(:schedule) { schedules(:child_schedule) }

    it 'permits viewing and administering' do
      aggregate_failures do
        expect(policy.index?).to be true
        expect(policy.show?).to be true
        expect(policy.create?).to be false
        expect(policy.new?).to be false
        expect(policy.update?).to be false
        expect(policy.edit?).to be false
        expect(policy.destroy?).to be false
        expect(policy.take_medication?).to be true
      end
    end
  end

  describe 'for parent without child' do
    let(:current_user) { users(:parent) }
    let(:schedule) { schedules(:adult_patient_schedule) }

    it 'forbids access' do
      expect(policy.show?).to be false
      expect(policy.take_medication?).to be false
    end
  end

  describe 'for adult patient with own schedule' do
    let(:current_user) { users(:adult_patient) }
    let(:schedule) { schedules(:adult_patient_schedule) }

    it 'permits viewing and taking own medication' do
      expect(policy.show?).to be true
      expect(policy.take_medication?).to be true
      expect(policy.create?).to be false
      expect(policy.update?).to be false
      expect(policy.destroy?).to be false
    end
  end

  describe 'for minor with own schedule' do
    let(:current_user) { users(:child_user) }
    let(:schedule) { schedules(:child_schedule) }

    it 'allows viewing own schedule' do
      expect(policy.show?).to be true
    end

    it 'does not allow taking medication without parent/carer' do
      expect(policy.take_medication?).to be false
    end

    it 'forbids management actions' do
      expect(policy.create?).to be false
      expect(policy.update?).to be false
      expect(policy.destroy?).to be false
    end
  end

  describe 'for nil user' do
    let(:current_user) { nil }

    it 'forbids all actions' do
      %i[index show create new update edit destroy take_medication].each do |action|
        expect(policy.public_send("#{action}?")).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { described_class::Scope.new(current_user, Schedule.all).resolve }

    context 'when user is an administrator' do
      let(:current_user) { users(:admin) }

      it 'returns all schedules' do
        expect(scope).to match_array(Schedule.all)
      end
    end

    context 'when user is a doctor' do
      let(:current_user) { users(:doctor) }

      it 'returns all schedules' do
        expect(scope).to match_array(Schedule.all)
      end
    end

    context 'when user is an adult patient' do
      let(:current_user) { users(:adult_patient) }

      it 'returns only their own schedules' do
        expect(scope).to contain_exactly(schedules(:adult_patient_schedule))
      end
    end

    context 'when user is a carer' do
      let(:current_user) { users(:carer) }

      it 'returns schedules for assigned patients' do
        expect(scope).to include(schedules(:patient_schedule))
        expect(scope).not_to include(schedules(:adult_patient_schedule))
      end
    end

    context 'when user is a parent' do
      let(:current_user) { users(:parent) }

      it 'returns schedules for their children only' do
        expect(scope).to include(schedules(:child_schedule))
        expect(scope).not_to include(schedules(:adult_patient_schedule))
      end
    end

    context 'when user is nil' do
      let(:current_user) { nil }

      it 'returns no schedules' do
        expect(scope).to be_empty
      end
    end
  end
end
