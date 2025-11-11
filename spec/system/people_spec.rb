# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People' do
  fixtures :users, :people, :medicines, :dosages, :prescriptions

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    sign_in(user)
  end

  describe 'index page' do
    it 'displays list of people with their prescription counts' do
      visit people_path

      within '[data-testid="people-list"]' do
        expect(page).to have_content('People')

        # Check that people are displayed
        people(:john).tap do |person|
          within "#person_#{person.id}" do
            expect(page).to have_content(person.name)
            expect(page).to have_content("Age: #{person.age}")
          end
        end
      end
    end

    it 'shows active prescription count for person with prescriptions' do
      person = people(:john)
      active_count = person.prescriptions.active.count

      visit people_path

      within "#person_#{person.id}" do
        if active_count.positive?
          expect(page).to have_content("#{active_count} active prescription")
        else
          expect(page).to have_content('No active prescriptions')
        end
      end
    end

    it 'shows no active prescriptions for person without prescriptions' do
      person = Person.create!(name: 'New Person', date_of_birth: 20.years.ago)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_content('No active prescriptions')
      end
    end

    it 'provides link to add prescription' do
      person = people(:john)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_link('Add Prescription', href: new_person_prescription_path(person))
      end
    end

    it 'provides link to view prescriptions when person has prescriptions' do
      person = people(:john)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_link('View Prescriptions', href: person_path(person)) if person.prescriptions.any?
      end
    end
  end

  describe 'patients without carers' do
    it 'highlights dependent adults who need carer assignment' do
      # Create a dependent adult without any carers
      patient_without_carer = Person.create!(
        name: 'Unassigned Patient',
        date_of_birth: 30.years.ago,
        person_type: :dependent_adult
      )

      visit people_path

      within "#person_#{patient_without_carer.id}" do
        expect(page).to have_css('[data-testid="needs-carer-badge"]')
        expect(page).to have_content('Needs Carer')
      end
    end

    it 'highlights minors who need carer assignment' do
      # Create a minor without any carers
      minor_without_carer = Person.create!(
        name: 'Unassigned Child',
        date_of_birth: 10.years.ago,
        person_type: :minor
      )

      visit people_path

      within "#person_#{minor_without_carer.id}" do
        expect(page).to have_css('[data-testid="needs-carer-badge"]')
        expect(page).to have_content('Needs Carer')
      end
    end

    it 'does not highlight self-managing adults without carers' do
      # Create a self-managing adult without carers
      adult_without_carer = Person.create!(
        name: 'Independent Adult',
        date_of_birth: 30.years.ago,
        person_type: :adult
      )

      visit people_path

      within "#person_#{adult_without_carer.id}" do
        expect(page).to have_no_css('[data-testid="needs-carer-badge"]')
      end
    end

    it 'does not highlight patients who have carers' do
      # John has a carer (nurse_smith) per fixtures
      person_with_carer = people(:john)

      visit people_path

      within "#person_#{person_with_carer.id}" do
        expect(page).to have_no_css('[data-testid="needs-carer-badge"]')
      end
    end
  end

  def sign_in(user)
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: 'password'
    click_button 'Sign in'
  end
end
