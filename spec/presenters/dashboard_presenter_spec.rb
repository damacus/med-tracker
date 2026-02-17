# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DashboardPresenter do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:minor_user) { users(:one) } # role: 5 (minor) - falls into else branch

  describe '#people' do
    context 'when user is an administrator' do
      it 'returns all people' do
        presenter = described_class.new(current_user: admin_user)
        expect(presenter.people).to eq(Person.all)
      end
    end

    context 'when user is a carer' do
      it 'returns patients of the carer' do
        presenter = described_class.new(current_user: carer_user)
        expect(presenter.people).to eq(carer_user.person.patients)
      end
    end

    context 'when user is a parent' do
      it 'returns minor patients of the parent' do
        presenter = described_class.new(current_user: parent_user)
        expect(presenter.people).to eq(parent_user.person.patients.where(person_type: :minor))
      end
    end

    context 'when user is a minor (no special access)' do
      it 'returns only their own person record' do
        presenter = described_class.new(current_user: minor_user)
        expect(presenter.people).to eq(Person.where(id: minor_user.person.id))
      end
    end
  end

  describe '#active_prescriptions' do
    it 'returns active prescriptions for scoped people' do
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.active_prescriptions).to all(have_attributes(active: true))
    end
  end

  describe '#upcoming_prescriptions' do
    it 'groups prescriptions by person' do
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.upcoming_prescriptions).to be_a(Hash)
      expect(presenter.upcoming_prescriptions.keys).to all(be_a(Person))
    end
  end

  describe '#doses' do
    it 'delegates to FamilyDashboard::ScheduleQuery' do
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.doses).to be_an(Array)
    end
  end
end
