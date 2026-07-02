# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :users, :dosages

  let(:household) { households(:fixture_household) }
  let(:current_user) { users(:jane) }
  let(:performance_budget_seconds) { 0.75 }

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
  end

  it 'renders a representative family dashboard within the performance budget' do
    people = create_list(:person, 8, household: household)
    people.each.with_index { |person, index| create_dashboard_records(person, index) }

    warm_presenter = dashboard_presenter(people)
    render_inline(described_class.new(presenter: warm_presenter))

    started_at = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    render_inline(described_class.new(presenter: dashboard_presenter(people)))
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started_at

    expect(elapsed).to be < performance_budget_seconds
  end

  def dashboard_presenter(people)
    DashboardPresenter.new(
      current_user: current_user,
      selected_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID,
      people_scope: Person.where(id: people.map(&:id)),
      household: household
    )
  end

  def create_dashboard_records(person, index)
    routine_medication = create(:medication, household: household, location: locations(:home))
    prn_medication = create(:medication, household: household, location: locations(:home))
    supplement = create(:medication, :vitamin, household: household, location: locations(:home))

    create_schedule(person, routine_medication, index)
    create_schedule(person, prn_medication, index, schedule_type: :prn)
    create_person_medication(person, supplement, index)
  end

  def create_schedule(person, medication, index, schedule_type: :daily)
    create(
      :schedule,
      person: person,
      medication: medication,
      dosage: nil,
      schedule_type: schedule_type,
      dose_amount: 500 + index,
      dose_unit: 'mg',
      start_date: Time.zone.today - 1.day,
      max_daily_doses: 4,
      min_hours_between_doses: 4
    )
  end

  def create_person_medication(person, medication, index)
    create(
      :person_medication,
      :routine,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: 1000 + index,
      dose_unit: 'IU',
      max_daily_doses: 1
    )
  end
end
