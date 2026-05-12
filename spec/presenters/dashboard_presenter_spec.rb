# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DashboardPresenter do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules, :person_medications,
           :medication_takes

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:minor_user) { users(:one) } # role: 5 (minor) - falls into else branch
  let(:userless_person_user) { users(:carer) }

  describe '#people' do
    context 'when user is an administrator' do
      it 'returns all people' do
        presenter = described_class.new(current_user: admin_user, selected_person_id: 'all')
        expect(presenter.people).to eq(Person.all)
      end
    end

    context 'when user is a carer' do
      it 'returns patients of the carer' do
        presenter = described_class.new(current_user: carer_user, selected_person_id: 'all')
        expect(presenter.people).to eq(carer_user.person.patients)
      end

      it 'returns Person.none when carer has no associated person' do
        allow(carer_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: carer_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when user is a parent' do
      it 'returns the parent and their minor patients' do
        presenter = described_class.new(current_user: parent_user, selected_person_id: 'all')
        parent_minor_ids = parent_user.person.patients.where(person_type: :minor).pluck(:id)
        expected_people = Person.where(id: [parent_user.person.id] + parent_minor_ids)
        expect(presenter.people).to eq(expected_people)
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
      it 'handles missing person record gracefully' do
        allow(carer_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: carer_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when parent user has no associated person' do
      it 'handles missing person record gracefully' do
        allow(parent_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: parent_user)
        expect(presenter.people).to eq(Person.none)
      end
    end

    context 'when non-privileged user has no associated person' do
      it 'handles missing person record gracefully' do
        allow(minor_user).to receive(:person).and_return(nil)
        presenter = described_class.new(current_user: minor_user)
        expect(presenter.people).to eq(Person.none)
      end
    end
  end

  describe 'dashboard person selection' do
    it 'defaults to the logged-in user when they are selectable' do
      presenter = described_class.new(current_user: parent_user)

      expect(presenter.selected_person).to eq(parent_user.person)
      expect(presenter.people).to eq([parent_user.person])
    end

    it 'defaults to the first available person when the logged-in user is not selectable' do
      presenter = described_class.new(current_user: carer_user)

      expect(presenter.selected_person).to eq(people(:child_patient))
      expect(presenter.people).to eq([people(:child_patient)])
    end

    it 'uses the all-family scope when requested' do
      presenter = described_class.new(current_user: parent_user, selected_person_id: 'all')

      expected_people = [parent_user.person] + parent_user.person.patients.where(person_type: :minor).to_a
      expect(presenter.selected_person).to be_nil
      expect(presenter.people).to eq(expected_people)
    end

    it 'filters active schedules to the selected person only' do
      presenter = described_class.new(current_user: parent_user, selected_person_id: people(:child_user_person).id)

      expect(presenter.people).to eq([people(:child_user_person)])
      expect(presenter.active_schedules).to include(schedules(:child_schedule))
      expect(presenter.active_schedules).not_to include(schedules(:jane_ibuprofen))
    end

    it 'keeps all selectable people available for the selector while the dashboard is filtered' do
      presenter = described_class.new(current_user: parent_user, selected_person_id: people(:child_user_person).id)

      expect(presenter.selectable_people).to include(parent_user.person, people(:child_user_person))
      expect(presenter.people).to eq([people(:child_user_person)])
    end

    it 'exposes individual people plus all-family selector options' do
      presenter = described_class.new(current_user: parent_user)

      labels = presenter.dashboard_person_options.map { |option| option.fetch(:label) }
      expect(labels).to include(parent_user.person.name, people(:child_user_person).name, 'All Family')
    end
  end

  describe 'private people scope assembly' do
    it 'builds an eager-loaded scope for explicit person ids' do
      presenter = described_class.new(current_user: parent_user)

      scoped_people = presenter.send(:people_scope, [parent_user.person.id, parent_user.person.id]).to_a

      expect(scoped_people.map(&:id)).to eq([parent_user.person.id])
      expect(scoped_people.first.association(:user)).to be_loaded
      expect(scoped_people.first.association(:schedules)).to be_loaded
      expect(scoped_people.first.association(:person_medications)).to be_loaded
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

  describe '#smart_insights' do
    it 'bulk loads dashboard insight sources for scoped people' do
      baseline_counts = count_insight_source_queries { described_class.new(current_user: admin_user).smart_insights }

      create_list(:person, 2).each do |person|
        medication = create(:medication, current_supply: 500, supply_at_last_restock: 500)
        create(:schedule, person: person, medication: medication)
      end

      expanded_counts = count_insight_source_queries { described_class.new(current_user: admin_user).smart_insights }

      expect(expanded_counts[:schedules]).to eq(baseline_counts[:schedules])
      expect(expanded_counts[:person_medications]).to eq(baseline_counts[:person_medications])
      expect(expanded_counts[:medication_takes]).to eq(baseline_counts[:medication_takes])
    end
  end

  def count_insight_source_queries(&)
    counts = Hash.new(0)

    subscriber = lambda do |_name, _start, _finish, _id, payload|
      sql = payload[:sql]
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      counts[:schedules] += 1 if sql.include?('FROM "schedules"')
      counts[:person_medications] += 1 if sql.include?('FROM "person_medications"')
      counts[:medication_takes] += 1 if sql.include?('FROM "medication_takes"')
    end

    ActiveRecord::Base.uncached do
      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    end
    counts
  end
end
