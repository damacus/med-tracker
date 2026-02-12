# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Takes Authorization', browser: false do
  fixtures :all

  before do
    driven_by(:rack_test)
  end

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }
  let(:carer) { users(:carer) }
  let(:parent) { users(:parent) }
  let(:adult_patient) { users(:adult_patient) }

  let(:adult_prescription) { prescriptions(:adult_patient_prescription) }

  describe 'viewing prescriptions to record medication takes' do
    context 'when user is authorized' do
      it 'allows administrators to view any prescription' do
        sign_in(admin)
        visit person_path(adult_prescription.person)

        # Should be able to see the prescription
        expect(page).to have_content(adult_prescription.medicine.name)
        expect(page).to have_no_content('You are not authorized')
      end

      it 'allows doctors to view any prescription' do
        sign_in(doctor)
        visit person_path(adult_prescription.person)

        expect(page).to have_content(adult_prescription.medicine.name)
        expect(page).to have_no_content('You are not authorized')
      end

      it 'allows nurses to view any prescription' do
        sign_in(nurse)
        visit person_path(adult_prescription.person)

        expect(page).to have_content(adult_prescription.medicine.name)
        expect(page).to have_no_content('You are not authorized')
      end

      it 'allows carers to view prescriptions for assigned patients' do
        sign_in(carer)
        # Carer is assigned to child_patient via carer_cares_for_patient fixture
        visit person_path(people(:child_patient))

        expect(page).to have_content('Child Patient')
        expect(page).to have_no_content('You are not authorized')
      end

      it 'allows parents to view prescriptions for their minor children' do
        sign_in(parent)
        # Parent is assigned to child_user_person via parent_cares_for_child fixture
        visit person_path(people(:child_user_person))

        expect(page).to have_content('Child User')
        expect(page).to have_no_content('You are not authorized')
      end

      it 'allows adult patients to view their own prescriptions' do
        sign_in(adult_patient)
        visit person_path(adult_patient.person)

        expect(page).to have_content('Adult Patient')
        expect(page).to have_no_content('You are not authorized')
      end
    end

    context 'when user is not authorized' do
      it 'denies carers from viewing prescriptions for unassigned patients' do
        sign_in(carer)
        # Try to access an unrelated person
        visit person_path(people(:john))

        expect(page).to have_content('You are not authorized')
      end

      it 'denies parents from viewing prescriptions for non-children' do
        sign_in(parent)
        # Try to access an adult patient
        visit person_path(people(:adult_patient_person))

        expect(page).to have_content('You are not authorized')
      end

      it 'denies adult patients from viewing others prescriptions' do
        sign_in(adult_patient)
        # Try to access another person
        visit person_path(people(:child_user_person))

        expect(page).to have_content('You are not authorized')
      end
    end
  end
end
