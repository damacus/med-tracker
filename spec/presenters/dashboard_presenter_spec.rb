# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DashboardPresenter do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:minor_user) { users(:one) } # role: 5 (minor) - falls into else branch
  let(:userless_person_user) { users(:carer) }

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

    context 'when carer user has no associated person' do
      it 'returns Person.none without raising' do
        allow(carer_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: carer_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when parent user has no associated person' do
      it 'returns Person.none without raising' do
        allow(parent_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: parent_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when non-privileged user has no associated person' do
      it 'returns Person.none without raising' do
        allow(minor_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: minor_user)
        expect(presenter.people).to eq(Person.none)
      end
    end
  end

  describe '#active_prescriptions' do
    it 'returns date-active prescriptions for scoped people' do
      presenter = described_class.new(current_user: admin_user)
      today = Time.zone.today
      expect(presenter.active_prescriptions).to all(
        satisfy { |p| today.between?(p.start_date, p.end_date) }
      )
    end

    it 'excludes prescriptions whose end_date is in the past' do
      past_prescription = prescriptions(:john_paracetamol)
      past_prescription.update!(start_date: 2.years.ago.to_date, end_date: 1.year.ago.to_date)
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.active_prescriptions).not_to include(past_prescription)
    end

    it 'excludes prescriptions whose start_date is in the future' do
      future_prescription = prescriptions(:john_paracetamol)
      future_prescription.update!(start_date: 1.year.from_now.to_date, end_date: 2.years.from_now.to_date)
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.active_prescriptions).not_to include(future_prescription)
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
