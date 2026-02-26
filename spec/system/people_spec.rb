# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :carer_relationships

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    sign_in(user)
  end

  describe 'index page' do
    it 'displays list of people with their schedule counts' do
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

    it 'shows active schedule count for person with schedules' do
      person = people(:john)
      active_count = person.schedules.active.count

      visit people_path

      within "#person_#{person.id}" do
        if active_count.positive?
          expect(page).to have_content("#{active_count} active schedule")
        else
          expect(page).to have_content('No active schedules')
        end
      end
    end

    it 'shows no active schedules for person without schedules' do
      person = Person.create!(name: 'New Person', date_of_birth: 20.years.ago)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_content('No active schedules')
      end
    end

    it 'provides link to add schedule' do
      person = people(:john)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_link('Add Schedule', href: new_person_schedule_path(person))
      end
    end

    it 'provides link to view schedules when person has schedules' do
      person = people(:john)

      visit people_path

      within "#person_#{person.id}" do
        expect(page).to have_link('View Schedules', href: person_path(person)) if person.schedules.any?
      end
    end
  end

  describe 'patients without carers' do
    it 'highlights dependent adults who need carer assignment' do
      carer = Person.create!(name: 'Temp Carer', date_of_birth: 40.years.ago, person_type: :adult)
      patient_without_carer = Person.new(
        name: 'Unassigned Patient',
        date_of_birth: 30.years.ago,
        person_type: :dependent_adult
      )
      patient_without_carer.carer_relationships.build(carer: carer, relationship_type: 'guardian')
      patient_without_carer.save!
      patient_without_carer.carer_relationships.destroy_all

      visit people_path

      within "#person_#{patient_without_carer.id}" do
        expect(page).to have_css('[data-testid="needs-carer-badge"]')
        expect(page).to have_content('Needs Carer')
      end
    end

    it 'highlights minors who need carer assignment' do
      carer = Person.create!(name: 'Temp Carer', date_of_birth: 40.years.ago, person_type: :adult)
      minor_without_carer = Person.new(
        name: 'Unassigned Child',
        date_of_birth: 10.years.ago,
        person_type: :minor
      )
      minor_without_carer.carer_relationships.build(carer: carer, relationship_type: 'parent')
      minor_without_carer.save!
      minor_without_carer.carer_relationships.destroy_all

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

  describe 'restricted action visibility' do
    context 'when user is an administrator' do
      let(:user) { users(:admin) }

      it 'shows Add Schedule action on person show page' do
        visit person_path(people(:child_patient))

        expect(page).to have_link('Add Schedule')
      end

      it 'shows Assign Carer action for unassigned dependent patients' do
        carer = Person.create!(name: 'Temp Carer', date_of_birth: 45.years.ago, person_type: :adult)
        patient_without_carer = Person.new(
          name: 'Unassigned Dependent',
          date_of_birth: 40.years.ago,
          person_type: :dependent_adult
        )
        patient_without_carer.carer_relationships.build(carer: carer, relationship_type: 'guardian')
        patient_without_carer.save!
        patient_without_carer.carer_relationships.destroy_all

        visit people_path

        within "#person_#{patient_without_carer.id}" do
          expect(page).to have_link('Assign Carer')
        end
      end
    end

    context 'when user is a carer' do
      let(:user) { users(:carer) }

      it 'hides Add Schedule action on person show page' do
        visit person_path(people(:child_patient))

        expect(page).to have_content('Care Actions')
        expect(page).to have_no_link('Add Schedule')
      end

      it 'hides Assign Carer action for unassigned dependent patients' do
        carer = Person.create!(name: 'Temp Carer', date_of_birth: 45.years.ago, person_type: :adult)
        patient_without_carer = Person.new(
          name: 'Unassigned Dependent',
          date_of_birth: 40.years.ago,
          person_type: :dependent_adult
        )
        patient_without_carer.carer_relationships.build(carer: carer, relationship_type: 'guardian')
        patient_without_carer.save!
        patient_without_carer.carer_relationships.destroy_all

        visit people_path

        expect(page).to have_content('People')
        expect(page).to have_no_css("#person_#{patient_without_carer.id}")
      end
    end

    context 'when user is a doctor' do
      let(:user) { users(:doctor) }

      it 'hides Assign Carer action for unassigned dependent patients' do
        carer = Person.create!(name: 'Temp Carer', date_of_birth: 45.years.ago, person_type: :adult)
        patient_without_carer = Person.new(
          name: 'Unassigned Dependent',
          date_of_birth: 40.years.ago,
          person_type: :dependent_adult
        )
        patient_without_carer.carer_relationships.build(carer: carer, relationship_type: 'guardian')
        patient_without_carer.save!
        patient_without_carer.carer_relationships.destroy_all

        visit people_path

        within "#person_#{patient_without_carer.id}" do
          expect(page).to have_no_link('Assign Carer')
        end
      end
    end
  end
end
