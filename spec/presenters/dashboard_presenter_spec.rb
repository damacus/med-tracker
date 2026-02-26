# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DashboardPresenter do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

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

      it 'returns Person.none when carer has no associated person' do
        allow(carer_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: carer_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when user is a parent' do
      it 'returns minor patients of the parent' do
        presenter = described_class.new(current_user: parent_user)
        expect(presenter.people).to eq(parent_user.person.patients.where(person_type: :minor))
      end

      it 'returns Person.none when parent has no associated person' do
        allow(parent_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: parent_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when user is a minor (no special access)' do
      it 'returns only their own person record' do
        presenter = described_class.new(current_user: minor_user)
        expect(presenter.people).to eq(Person.where(id: minor_user.person.id))
      end

      it 'returns Person.none when minor has no associated person' do
        allow(minor_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: minor_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when current_user is nil' do
      it 'returns Person.none' do
        presenter = described_class.new(current_user: nil)
        expect(presenter.people).to eq(Person.none)
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

  describe '#active_schedules' do
    it 'returns date-active schedules for scoped people' do
      presenter = described_class.new(current_user: admin_user)
      today = Time.zone.today
      expect(presenter.active_schedules).to all(
        satisfy { |p| today.between?(p.start_date, p.end_date) }
      )
    end

    it 'excludes schedules whose end_date is in the past' do
      past_schedule = schedules(:john_paracetamol)
      past_schedule.update!(start_date: 2.years.ago.to_date, end_date: 1.year.ago.to_date)
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.active_schedules).not_to include(past_schedule)
    end

    it 'excludes schedules whose start_date is in the future' do
      future_schedule = schedules(:john_paracetamol)
      future_schedule.update!(start_date: 1.year.from_now.to_date, end_date: 2.years.from_now.to_date)
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.active_schedules).not_to include(future_schedule)
    end
  end

  describe '#upcoming_schedules' do
    it 'groups schedules by person' do
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.upcoming_schedules).to be_a(Hash)
      expect(presenter.upcoming_schedules.keys).to all(be_a(Person))
    end
  end

  describe '#doses' do
    it 'delegates to FamilyDashboard::ScheduleQuery' do
      presenter = described_class.new(current_user: admin_user)
      expect(presenter.doses).to be_an(Array)
    end
  end
end
