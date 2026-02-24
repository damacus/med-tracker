# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard Authorization', type: :system do
  fixtures :accounts, :people, :users, :prescriptions, :locations, :medicines, :dosages, :carer_relationships

  describe 'viewing the dashboard' do
    context 'when signed in as an administrator' do
      let(:admin) { users(:admin) }

      it 'sees all people and prescriptions' do
        sign_in(admin)

        visit dashboard_path

        # Should see multiple people
        expect(page).to have_content('John Doe')
        expect(page).to have_content('Jane Doe')
        expect(page).to have_content('Bob Smith')
        expect(page).to have_content('Adult Patient')
        expect(page).to have_content('Child Patient')
      end
    end

    context 'when signed in as a doctor' do
      let(:doctor) { users(:doctor) }

      it 'sees all people and prescriptions' do
        sign_in(doctor)

        visit dashboard_path

        # Should see multiple people
        expect(page).to have_content('John Doe')
        expect(page).to have_content('Jane Doe')
        expect(page).to have_content('Bob Smith')
        expect(page).to have_content('Adult Patient')
      end
    end

    context 'when signed in as a nurse' do
      let(:nurse) { users(:nurse) }

      it 'sees all people and prescriptions' do
        sign_in(nurse)

        visit dashboard_path

        # Should see multiple people
        expect(page).to have_content('John Doe')
        expect(page).to have_content('Jane Doe')
        expect(page).to have_content('Bob Smith')
      end
    end

    context 'when signed in as a carer' do
      let(:carer) { users(:carer) }

      it 'sees only assigned patients' do
        sign_in(carer)

        visit dashboard_path

        # Should see assigned patients
        expect(page).to have_content('Child Patient')
        expect(page).to have_content('Child User')

        # Should NOT see unassigned patients
        expect(page).to have_no_content('Bob Smith')
        expect(page).to have_no_content('John Doe')
        expect(page).to have_no_content('Jane Doe')
      end

      it 'sees only prescriptions for assigned patients' do
        sign_in(carer)

        visit dashboard_path

        # Should see prescriptions for assigned patients
        # child_patient has ibuprofen prescription
        expect(page).to have_content('Ibuprofen')

        # Should NOT see Bob's aspirin prescription
        expect(page).to have_no_content('Aspirin')
      end
    end

    context 'when signed in as a parent' do
      let(:parent) { users(:parent) }

      it 'sees only their minor children' do
        sign_in(parent)

        visit dashboard_path

        # Should see their child
        expect(page).to have_content('Child User')

        # Should NOT see other people
        expect(page).to have_no_content('Bob Smith')
        expect(page).to have_no_content('John Doe')
        expect(page).to have_no_content('Jane Doe')
        expect(page).to have_no_content('Adult Patient')
      end

      it 'sees only prescriptions for their children' do
        sign_in(parent)

        visit dashboard_path

        # Should see child's prescription
        expect(page).to have_content('Paracetamol')

        # Should NOT see other prescriptions
        expect(page).to have_no_content('Aspirin')
      end
    end

    context 'when signed in as an adult patient' do
      let(:adult_patient) { users(:adult_patient) }

      it 'sees only themselves' do
        sign_in(adult_patient)

        visit dashboard_path

        # Should see themselves
        expect(page).to have_content('Adult Patient')

        # Should NOT see other people
        expect(page).to have_no_content('Bob Smith')
        expect(page).to have_no_content('John Doe')
        expect(page).to have_no_content('Jane Doe')
        expect(page).to have_no_content('Child Patient')
      end

      it 'sees only their own prescriptions' do
        sign_in(adult_patient)

        visit dashboard_path

        # Should see their own prescription (paracetamol)
        expect(page).to have_content('Paracetamol')

        # Should NOT see other prescriptions
        expect(page).to have_no_content('Aspirin')
        expect(page).to have_no_content('Ibuprofen')
      end
    end
  end
end
