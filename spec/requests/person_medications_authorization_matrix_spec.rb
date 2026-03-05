# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medications authorization matrix' do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }
  let(:carer) { users(:carer) }
  let(:parent) { users(:parent) }
  let(:linked_child) { people(:child_user_person) }
  let(:assigned_patient) { people(:child_patient) }
  let(:unrelated_person) { people(:john) }
  let(:medication) { medications(:vitamin_d) }

  describe 'POST /people/:person_id/person_medications' do
    it 'allows administrators to create medications for any person' do
      sign_in(admin)

      expect do
        post person_person_medications_path(assigned_patient),
             params: { person_medication: { medication_id: medication.id, notes: 'Admin create' } }
      end.to change(PersonMedication, :count).by(1)

      expect(response).to redirect_to(person_path(assigned_patient))
    end

    it 'allows parents to create medications for linked children' do
      sign_in(parent)

      expect do
        post person_person_medications_path(linked_child),
             params: { person_medication: { medication_id: medication.id, notes: 'Parent create' } }
      end.to change(PersonMedication, :count).by(1)

      expect(response).to redirect_to(person_path(linked_child))
    end

    it 'denies parents from creating medications for unlinked children' do
      sign_in(parent)

      expect do
        post person_person_medications_path(assigned_patient),
             params: { person_medication: { medication_id: medication.id, notes: 'Denied parent create' } }
      end.not_to change(PersonMedication, :count)

      expect(response).to redirect_to(root_path)
    end

    it 'denies carers from creating medications for assigned patients' do
      sign_in(carer)

      expect do
        post person_person_medications_path(assigned_patient),
             params: { person_medication: { medication_id: medication.id, notes: 'Denied carer create' } }
      end.not_to change(PersonMedication, :count)

      expect(response).to redirect_to(root_path)
    end

    it 'denies nurses from creating medications' do
      sign_in(nurse)

      expect do
        post person_person_medications_path(assigned_patient),
             params: { person_medication: { medication_id: medication.id, notes: 'Denied nurse create' } }
      end.not_to change(PersonMedication, :count)

      expect(response).to redirect_to(root_path)
    end
  end

  describe 'POST /people/:person_id/person_medications/:id/take_medication' do
    let!(:assigned_person_medication) do
      PersonMedication.create!(person: assigned_patient, medication: medication, notes: 'Assigned person medication')
    end

    it 'allows carers to take medication for assigned patients' do
      sign_in(carer)

      expect do
        post take_medication_person_person_medication_path(assigned_patient, assigned_person_medication)
      end.to change(MedicationTake, :count).by(1)

      expect(response).to redirect_to(person_path(assigned_patient))
    end

    it 'denies doctors from taking medication' do
      sign_in(doctor)

      expect do
        post take_medication_person_person_medication_path(assigned_patient, assigned_person_medication)
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(root_path)
    end

    it 'denies nurses from taking medication' do
      sign_in(nurse)

      expect do
        post take_medication_person_person_medication_path(assigned_patient, assigned_person_medication)
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(root_path)
    end

    it 'denies carers from taking medication for unrelated people' do
      unrelated_person_medication = PersonMedication.create!(
        person: unrelated_person,
        medication: medications(:ibuprofen),
        notes: 'Unrelated'
      )
      sign_in(carer)

      expect do
        post take_medication_person_person_medication_path(unrelated_person, unrelated_person_medication)
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(root_path)
    end
  end

  describe 'DELETE /people/:person_id/person_medications/:id' do
    it 'allows administrators to remove medications from any person' do
      person_medication = PersonMedication.create!(person: assigned_patient, medication: medication, notes: 'Delete me')
      sign_in(admin)

      expect do
        delete person_person_medication_path(assigned_patient, person_medication)
      end.to change(PersonMedication, :count).by(-1)

      expect(response).to redirect_to(person_path(assigned_patient))
    end

    it 'denies doctors from removing medications' do
      person_medication = PersonMedication.create!(person: assigned_patient, medication: medication, notes: 'No doctor delete')
      sign_in(doctor)

      expect do
        delete person_person_medication_path(assigned_patient, person_medication)
      end.not_to change(PersonMedication, :count)

      expect(response).to redirect_to(root_path)
    end

    it 'denies carers from removing medications for assigned patients' do
      person_medication = PersonMedication.create!(person: assigned_patient, medication: medication, notes: 'No carer delete')
      sign_in(carer)

      expect do
        delete person_person_medication_path(assigned_patient, person_medication)
      end.not_to change(PersonMedication, :count)

      expect(response).to redirect_to(root_path)
    end
  end

  describe 'PATCH /people/:person_id/person_medications/:id/reorder' do
    it 'allows parents to reorder medications for linked children' do
      first = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_d), notes: 'First')
      second = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_c), notes: 'Second')
      sign_in(parent)

      patch reorder_person_person_medication_path(linked_child, second), params: { direction: 'up' }

      expect(response).to redirect_to(person_path(linked_child))
      expect(linked_child.person_medications.order(:position, :id).pluck(:id)).to eq([second.id, first.id])
    end

    it 'denies carers from reordering medications for assigned patients' do
      first = PersonMedication.create!(person: assigned_patient, medication: medications(:ibuprofen), notes: 'First')
      second = PersonMedication.create!(person: assigned_patient, medication: medications(:aspirin), notes: 'Second')
      sign_in(carer)

      patch reorder_person_person_medication_path(assigned_patient, second), params: { direction: 'up' }

      expect(response).to redirect_to(root_path)
      expect(assigned_patient.person_medications.order(:position, :id).pluck(:id)).to eq([first.id, second.id])
    end
  end
end
