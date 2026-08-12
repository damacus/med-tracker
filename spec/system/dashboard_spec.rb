# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard', :browser do
  fixtures :accounts, :users, :locations, :medications, :dosages, :schedules, :people,
           :carer_relationships, :person_medications, :medication_takes

  it 'loads the dashboard and allows taking a dose from the timeline' do
    driven_by(:playwright)

    travel_to(Time.current.beginning_of_day + 9.hours) do
      sign_in(users(:jane))
      visit dashboard_path

      expect(page).to have_text('Good morning')
      expect(page).to have_text("Today's Schedule")
      expect(page).to have_text('Ibuprofen')
      expect(page).to have_text('Jane Doe')
      expect(page).to have_text('Child Patient')

      button = first('[data-testid^="take-dose-"]')

      expect do
        button.click
        within(first("form[action*='take_medication']", visible: :all)) do
          click_button I18n.t('person_medications.card.take')
        end
        expect(page).to have_text('Medication taken successfully.', wait: 10)
      end.to change(MedicationTake, :count).by(1)
    end
  end

  it 'refreshes the dashboard aggregates after recording a routine dose without a reload' do
    driven_by(:playwright)
    person = people(:jane)
    medication = medications(:movicol)
    MedicationTake.delete_all
    Schedule.where(person: person).delete_all
    PersonMedication.where(person: person).delete_all
    medication.update!(current_supply: 3)
    schedule = Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 1,
      dose_unit: 'sachet',
      frequency: 'Once daily',
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      max_daily_doses: 1,
      schedule_config: { 'times' => ['09:00'] }
    )

    travel_to(Time.zone.now.change(hour: 10, min: 0)) do
      track_dashboard_document_loads
      sign_in(users(:jane))
      visit dashboard_path(dashboard_person_id: person.id)

      document_load_count = dashboard_document_load_count
      expect(dashboard_metric_value('DUE NOW')).to eq('1')
      expect(dashboard_metric_value('TASKS LEFT')).to eq('1')
      expect(page).to have_text('1 routine task left today')
      expect(page).to have_css('[data-testid="dashboard-stock-meter"][aria-label="3 units left"]')

      find("[data-testid='take-dose-schedule_#{schedule.id}']").click
      within("form[action='#{take_medication_person_schedule_path(person, schedule)}']") do
        click_button 'Take'
      end

      expect(page).to have_text('Medication taken successfully.')
      expect(dashboard_document_load_count).to eq(document_load_count)
      expect(page).to have_text(/Taken/i)
      expect(dashboard_metric_value('NEXT DUE')).to eq('None today')
      expect(dashboard_metric_value('DUE NOW')).to eq('0')
      expect(dashboard_metric_value('TASKS LEFT')).to eq('0')
      expect(page).to have_text('Routine tasks done today')
      expect(page).to have_css('[data-testid="dashboard-stock-meter"][aria-label="2 units left"]')
      expect(medication.reload.current_supply).to eq(2)
    end
  end

  it 'keeps the time grouping after recording a family-lanes dose without a reload' do
    driven_by(:playwright)
    person = people(:jane)
    medication = medications(:movicol)
    account = accounts(:jane_doe)
    MedicationTake.delete_all
    Schedule.where(person: person).delete_all
    PersonMedication.where(person: person).delete_all
    account.update!(dashboard_variant: 'family_lanes')
    schedule = Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 1,
      dose_unit: 'sachet',
      frequency: 'Once daily',
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      max_daily_doses: 1,
      schedule_config: { 'times' => ['09:00'] }
    )

    travel_to(Time.zone.now.change(hour: 10, min: 0)) do
      sign_in(users(:jane))
      visit dashboard_path(dashboard_grouping: 'time')

      expect(page).to have_css('[data-testid="dashboard-family-time"]')
      find("[data-testid='take-dose-schedule_#{schedule.id}']").click
      within("form[action^='#{take_medication_person_schedule_path(person, schedule)}']") do
        expect(page).to have_field('dashboard_grouping', type: 'hidden', with: 'time')
        click_button 'Take'
      end

      expect(page).to have_text('Medication taken successfully.')
      expect(page).to have_css('[data-testid="dashboard-family-time"]')
      expect(page).to have_text(/Taken/i)
    end
  end

  it 'refreshes the dashboard aggregates after recording a direct medication dose without a reload' do
    driven_by(:playwright)
    person = people(:jane)
    medication = medications(:movicol)
    MedicationTake.delete_all
    Schedule.where(person: person).delete_all
    PersonMedication.where(person: person).delete_all
    medication.update!(current_supply: 3)
    person_medication = PersonMedication.create!(
      person: person,
      medication: medication,
      dose_amount: 1,
      dose_unit: 'sachet',
      administration_kind: :routine,
      max_daily_doses: 1
    )

    travel_to(Time.zone.now.change(hour: 10, min: 0)) do
      track_dashboard_document_loads
      sign_in(users(:jane))
      visit dashboard_path(dashboard_person_id: person.id)

      document_load_count = dashboard_document_load_count
      expect(dashboard_metric_value('DUE NOW')).to eq('1')
      expect(dashboard_metric_value('TASKS LEFT')).to eq('1')
      expect(page).to have_text('1 routine task left today')
      expect(page).to have_css('[data-testid="dashboard-stock-meter"][aria-label="3 units left"]')

      find("[data-testid='take-dose-personmedication_#{person_medication.id}']").click
      within("form[action='#{take_medication_person_person_medication_path(person, person_medication)}']") do
        click_button 'Take'
      end

      expect(page).to have_text('Medication taken successfully.')
      expect(dashboard_document_load_count).to eq(document_load_count)
      expect(dashboard_metric_value('NEXT DUE')).to eq('None today')
      expect(dashboard_metric_value('DUE NOW')).to eq('0')
      expect(dashboard_metric_value('TASKS LEFT')).to eq('0')
      expect(page).to have_text('Routine tasks done today')
      expect(page).to have_css('[data-testid="dashboard-stock-meter"][aria-label="2 units left"]')
      within('#dashboard [data-testid="dashboard-today-dose-history"]') do
        expect(page).to have_text(medication.display_name)
      end
      expect(MedicationTake.find_by(person_medication: person_medication)).to be_present
      expect(medication.reload.current_supply).to eq(2)
    end
  end

  it 'keeps all-family selected after recording a scheduled dose without a reload' do
    driven_by(:playwright)
    person = people(:jane)
    other_person = people(:child_patient)
    MedicationTake.delete_all
    Schedule.delete_all
    PersonMedication.delete_all
    accounts(:jane_doe).update!(dashboard_variant: 'current')
    schedule = create_dashboard_schedule(person, medications(:movicol))
    other_schedule = create_dashboard_schedule(other_person, medications(:ibuprofen))

    travel_to(Time.zone.now.change(hour: 10, min: 0)) do
      sign_in(users(:jane))
      visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)

      find("[data-testid='take-dose-schedule_#{schedule.id}']").click
      within("form[action='#{take_medication_person_schedule_path(person, schedule)}']") do
        expect(page).to have_field(
          'dashboard_person_id',
          type: 'hidden',
          with: DashboardPresenter::ALL_FAMILY_PERSON_ID
        )
        click_button 'Take'
      end

      expect(page).to have_text('Medication taken successfully.')
      find("[data-testid='take-dose-schedule_#{other_schedule.id}']").click
      within("form[action='#{take_medication_person_schedule_path(other_person, other_schedule)}']") do
        expect(page).to have_field(
          'dashboard_person_id',
          type: 'hidden',
          with: DashboardPresenter::ALL_FAMILY_PERSON_ID
        )
      end
    end
  end

  it 'keeps all-family selected after recording a direct medication dose without a reload' do
    driven_by(:playwright)
    person = people(:jane)
    other_person = people(:child_patient)
    MedicationTake.delete_all
    Schedule.delete_all
    PersonMedication.delete_all
    accounts(:jane_doe).update!(dashboard_variant: 'current')
    person_medication = create_dashboard_person_medication(person, medications(:movicol))
    other_person_medication = create_dashboard_person_medication(other_person, medications(:ibuprofen))

    travel_to(Time.zone.now.change(hour: 10, min: 0)) do
      sign_in(users(:jane))
      visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)

      find("[data-testid='take-dose-personmedication_#{person_medication.id}']").click
      within("form[action='#{take_medication_person_person_medication_path(person, person_medication)}']") do
        expect(page).to have_field(
          'dashboard_person_id',
          type: 'hidden',
          with: DashboardPresenter::ALL_FAMILY_PERSON_ID
        )
        click_button 'Take'
      end

      expect(page).to have_text('Medication taken successfully.')
      find("[data-testid='take-dose-personmedication_#{other_person_medication.id}']").click
      other_person_take_path = take_medication_person_person_medication_path(
        other_person,
        other_person_medication
      )
      within("form[action='#{other_person_take_path}']") do
        expect(page).to have_field(
          'dashboard_person_id',
          type: 'hidden',
          with: DashboardPresenter::ALL_FAMILY_PERSON_ID
        )
      end
    end
  end

  def dashboard_metric_value(label)
    metric = page.find_by_id('dashboard').all('p').find { |element| element.text == label }
    metric.find(:xpath, '../..').find('span').text
  end

  def track_dashboard_document_loads
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(script: <<~JS)
        const key = 'dashboard-document-loads';
        const loads = Number(sessionStorage.getItem(key) || '0');
        sessionStorage.setItem(key, String(loads + 1));
      JS
    end
  end

  def dashboard_document_load_count
    page.evaluate_script("Number(sessionStorage.getItem('dashboard-document-loads'))")
  end

  def create_dashboard_schedule(person, medication)
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 1,
      dose_unit: medication.dose_unit,
      frequency: 'Once daily',
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      max_daily_doses: 1,
      schedule_config: { 'times' => ['09:00'] }
    )
  end

  def create_dashboard_person_medication(person, medication)
    PersonMedication.create!(
      person: person,
      medication: medication,
      dose_amount: 1,
      dose_unit: medication.dose_unit,
      administration_kind: :routine,
      max_daily_doses: 1
    )
  end
end
