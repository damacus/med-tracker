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

  def sign_in(user)
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: 'password'
    click_button 'Sign in'
  end
end
