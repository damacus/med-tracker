# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medications', type: :system do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :users, :person_medications

  let(:person) { people(:john) }
  let(:user) { users(:john) }

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
    login_as(user)
  end

  describe 'adding a medication assignment' do
    let(:new_medication) { medications(:vitamin_c) }

    it 'allows adding a medication from predefined defaults', :js do
      visit person_path(person)

      expect(page).to have_text('Medications')

      within '[data-testid="quick-actions"]' do
        click_link 'Add Medication'
      end

      click_link 'As needed'
      click_button 'Select a medication'
      find('label', text: new_medication.name).click

      expect(page).to have_text('Choose the dose')
      expect(page).to have_css('#person_medication_dose_option option', text: '500 mg', visible: :all)
      select '500 mg', from: 'Dose'
      click_button 'Next'

      expect(page).to have_text('Additional guidance')

      click_button 'Add Medication'

      expect(page).to have_text('Medication added successfully.')
      expect(page).to have_text(new_medication.name)
    end
  end

  describe 'editing an existing medication assignment' do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    it 'opens the edit modal from the card', :js do
      visit person_path(person)

      within("##{tenant_dom_id(person_medication)}") do
        find("[data-testid='person-medication-actions-#{person_medication.id}']").click
        find("[data-testid='edit-person-medication-#{person_medication.id}']").click
      end

      expect(page).to have_text(/edit medication/i)
      expect(page).to have_css('div[data-state="open"]')
    end
  end
end
