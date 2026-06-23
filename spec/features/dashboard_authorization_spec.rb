# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard Authorization', type: :system do
  fixtures :accounts, :people, :users, :schedules, :locations, :medications, :dosages, :carer_relationships

  describe 'viewing the dashboard' do
    context 'when signed in as an administrator' do
      let(:admin) { users(:admin) }

      it 'sees all people and schedules' do
        sign_in(admin)

        visit dashboard_path

        # Should see multiple people
        expect_person_selector_to_include('John Doe', 'Jane Doe', 'Bob Smith', 'Adult Patient', 'Child Patient')
      end
    end

    context 'when signed in as a doctor' do
      let(:doctor) { users(:doctor) }

      it 'sees all people and schedules' do
        sign_in(doctor)

        visit dashboard_path

        # Should see multiple people
        expect_person_selector_to_include('John Doe', 'Jane Doe', 'Bob Smith', 'Adult Patient')
      end
    end

    context 'when signed in as a nurse' do
      let(:nurse) { users(:nurse) }

      it 'sees all people and schedules' do
        sign_in(nurse)

        visit dashboard_path

        # Should see multiple people
        expect_person_selector_to_include('John Doe', 'Jane Doe', 'Bob Smith')
      end
    end

    context 'when signed in as a carer' do
      let(:carer) { users(:carer) }

      it 'sees only assigned patients' do
        sign_in(carer)

        visit dashboard_path

        # Should see assigned patients
        expect(page).to have_text('Child Patient')
        expect(page).to have_text('Child User')
        expect_person_selector_to_include('Child Patient', 'Child User')

        # Should NOT see unassigned patients
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
        expect_person_selector_to_exclude('Bob Smith', 'John Doe', 'Jane Doe')
      end

      it 'sees only schedules for assigned patients' do
        sign_in(carer)

        visit dashboard_path
        click_link 'Child Patient'

        # Should see schedules for assigned patients
        # child_patient has ibuprofen schedule
        expect(page).to have_text('Ibuprofen')

        # Should NOT see Bob's aspirin schedule
        expect(page).to have_no_text('Aspirin')
      end
    end

    context 'when signed in as a parent' do
      let(:parent) { users(:parent) }

      it 'sees only their minor children' do
        sign_in(parent)

        visit dashboard_path

        # Should see their child
        expect(page).to have_text('Child User')
        expect_person_selector_to_include('Child User')

        # Should NOT see other people
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
        expect(page).to have_no_text('Adult Patient')
        expect_person_selector_to_exclude('Bob Smith', 'John Doe', 'Jane Doe', 'Adult Patient')
      end

      it 'sees only schedules for their children' do
        sign_in(parent)

        visit dashboard_path
        click_link 'Child User'
        open_as_needed_disclosures

        # Should see child's schedule
        expect(page).to have_text('Paracetamol')

        # Should NOT see other schedules
        expect(page).to have_no_text('Aspirin')
      end
    end

    context 'when signed in as an adult patient' do
      let(:adult_patient) { users(:adult_patient) }

      it 'sees only themselves' do
        sign_in(adult_patient)

        visit dashboard_path

        # Should see themselves
        expect(page).to have_text('Adult Patient')
        expect_person_selector_to_include('Adult Patient')

        # Should NOT see other people
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
        expect(page).to have_no_text('Child Patient')
        expect_person_selector_to_exclude('Bob Smith', 'John Doe', 'Jane Doe', 'Child Patient')
      end

      it 'sees only their own schedules' do
        sign_in(adult_patient)

        visit dashboard_path
        open_as_needed_disclosures

        # Should see their own schedule (paracetamol)
        expect(page).to have_text('Paracetamol')

        # Should NOT see other schedules
        expect(page).to have_no_text('Aspirin')
        expect(page).to have_no_text('Ibuprofen')
      end
    end
  end

  def open_as_needed_disclosures
    page.all('details[data-testid="dashboard-as-needed-person"] summary').to_a.each(&:click)
  end

  def expect_person_selector_to_include(*names)
    selector_text = find('[data-testid="dashboard-person-selector"]', visible: :all).text(:all)

    names.each do |name|
      expect(selector_text).to include(name)
    end
  end

  def expect_person_selector_to_exclude(*names)
    selector_text = find('[data-testid="dashboard-person-selector"]', visible: :all).text(:all)

    names.each do |name|
      expect(selector_text).not_to include(name)
    end
  end
end
