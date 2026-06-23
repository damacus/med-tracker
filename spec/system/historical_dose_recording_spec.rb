# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Historical dose recording' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages

  let(:person) { people(:jane) }
  let(:admin) { users(:admin) }
  let(:medication) { medications(:ibuprofen) }

  before do
    sign_in(admin)
  end

  it 'records a historical dose via the overflow menu prior-day dialog' do
    travel_to(Time.zone.local(2026, 4, 28, 14, 45)) do
      schedule = build_schedule
      build_alternate_medication
      submitted_time = Time.zone.local(2026, 4, 27, 8, 30)

      visit person_path(person)
      within("##{tenant_dom_id(schedule)}") do
        click_button 'Log a past dose'
      end

      expect(page).to have_text('Record a dose from a previous day')

      field = find("input[name='medication_take[taken_at]'][type='datetime-local']", visible: :all)
      expect(field[:value]).to eq('2026-04-28T14:45')
      expect(field[:max]).to eq('2026-04-28T14:45')

      expect do
        within("form[action='#{take_path(schedule)}']") do
          fill_in 'Date and time taken', with: submitted_time.strftime('%Y-%m-%dT%H:%M')
          click_button I18n.t('medications.prior_day_take_action.submit')
        end
        expect(page).to have_text(I18n.t('schedules.medication_taken'), wait: 10)
      end.to change(MedicationTake, :count).by(1)

      expect(MedicationTake.order(:id).last.taken_at).to be_within(1.second).of(submitted_time)
    end
  end

  def build_schedule
    Schedule.create!(
      person: person,
      medication: medication,
      dosage: dosages(:ibuprofen_adult),
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  def build_alternate_medication
    Medication.create!(
      name: medication.name,
      location: locations(:school),
      category: medication.category,
      dosage_amount: medication.dosage_amount,
      dosage_unit: medication.dosage_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end

  def take_path(schedule)
    take_medication_person_schedule_path(person, schedule)
  end
end
