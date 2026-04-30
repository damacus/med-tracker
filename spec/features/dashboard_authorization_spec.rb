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
        expect(page).to have_text('John Doe')
        expect(page).to have_text('Jane Doe')
        expect(page).to have_text('Bob Smith')
        expect(page).to have_text('Adult Patient')
        expect(page).to have_text('Child Patient')
      end
    end

    context 'when signed in as a doctor' do
      let(:doctor) { users(:doctor) }

      it 'sees all people and schedules' do
        sign_in(doctor)

        visit dashboard_path

        # Should see multiple people
        expect(page).to have_text('John Doe')
        expect(page).to have_text('Jane Doe')
        expect(page).to have_text('Bob Smith')
        expect(page).to have_text('Adult Patient')
      end
    end

    context 'when signed in as a nurse' do
      let(:nurse) { users(:nurse) }

      it 'sees all people and schedules' do
        sign_in(nurse)

        visit dashboard_path

        # Should see multiple people
        expect(page).to have_text('John Doe')
        expect(page).to have_text('Jane Doe')
        expect(page).to have_text('Bob Smith')
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

        # Should NOT see unassigned patients
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
      end

      it 'sees only schedules for assigned patients' do
        sign_in(carer)

        visit dashboard_path

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

        # Should NOT see other people
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
        expect(page).to have_no_text('Adult Patient')
      end

      it 'sees only schedules for their children' do
        sign_in(parent)

        visit dashboard_path

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

        # Should NOT see other people
        expect(page).to have_no_text('Bob Smith')
        expect(page).to have_no_text('John Doe')
        expect(page).to have_no_text('Jane Doe')
        expect(page).to have_no_text('Child Patient')
      end

      it 'sees only their own schedules' do
        sign_in(adult_patient)

        visit dashboard_path

        # Should see their own schedule (paracetamol)
        expect(page).to have_text('Paracetamol')

        # Should NOT see other schedules
        expect(page).to have_no_text('Aspirin')
        expect(page).to have_no_text('Ibuprofen')
      end
    end
  end
end
