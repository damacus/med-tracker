# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule dose cycle defaults' do
  fixtures :accounts, :users, :locations

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:location) { locations(:home) }
  let!(:person) { create(:person, name: 'Weekly Patient') }
  let!(:medication) { create(:medication, name: 'Weekly Capsule', location:, dosage_amount: nil, dosage_unit: nil) }
  let!(:dosage) do
    create(
      :dosage,
      medication:,
      amount: 1,
      unit: 'capsule',
      description: 'Weekly dose',
      frequency: 'Weekly default',
      default_max_daily_doses: 2,
      default_min_hours_between_doses: 12,
      default_dose_cycle: :weekly
    )
  end

  it 'applies and reads the selected dose cycle from the combobox radios' do
    dosage
    login_as(admin)
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_on 'Add Medication'
    end
    click_on 'Prescribed / Scheduled'

    find_by_id('medication_trigger').click
    find('label', text: medication.name, visible: :all, wait: 10).click

    expect(page).to have_content('Add Plan')
    find('label', text: '1.0 capsule', visible: :all, wait: 10).click

    expect(page).to have_css('input[name="schedule[dose_cycle]"][value="weekly"]:checked', visible: :hidden)

    fill_in 'Max doses per cycle', with: '3'

    expect(find_field('Frequency').value).to include('weekly')
    expect(find_field('Frequency').value).to include('12h')
  end
end
