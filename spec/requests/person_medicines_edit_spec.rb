# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medicine edit and update' do
  fixtures :accounts, :people, :users, :locations, :medicines, :carer_relationships

  let(:person) { people(:child_user_person) }
  let(:assigned_patient) { people(:child_patient) }

  let!(:person_medicine) do
    PersonMedicine.create!(
      person: person,
      medicine: medicines(:vitamin_d),
      notes: 'Original notes',
      max_daily_doses: 3,
      min_hours_between_doses: 4
    )
  end

  describe 'GET /people/:person_id/person_medicines/:id/edit' do
    context 'when signed in as admin' do
      before { sign_in(users(:admin)) }

      it 'returns 200' do
        get edit_person_person_medicine_path(person, person_medicine)
        expect(response).to have_http_status(:ok)
      end
    end

    context 'when signed in as parent of linked child' do
      before { sign_in(users(:parent)) }

      it 'returns 200' do
        get edit_person_person_medicine_path(person, person_medicine)
        expect(response).to have_http_status(:ok)
      end
    end

    context 'when signed in as carer' do
      before { sign_in(users(:carer)) }

      it 'redirects (not authorized)' do
        carer_medicine = PersonMedicine.create!(person: assigned_patient, medicine: medicines(:vitamin_d))
        get edit_person_person_medicine_path(assigned_patient, carer_medicine)
        expect(response).to redirect_to(root_path)
      end
    end
  end

  describe 'PATCH /people/:person_id/person_medicines/:id' do
    context 'when signed in as admin' do
      before { sign_in(users(:admin)) }

      it 'updates the person medicine and redirects' do
        patch person_person_medicine_path(person, person_medicine),
              params: { person_medicine: { notes: 'Updated notes', max_daily_doses: 5 } }

        expect(response).to redirect_to(person_path(person))
        person_medicine.reload
        expect(person_medicine.notes).to eq('Updated notes')
        expect(person_medicine.max_daily_doses).to eq(5)
      end

      it 'updates via Turbo Stream' do
        patch person_person_medicine_path(person, person_medicine),
              params: { person_medicine: { notes: 'Turbo update' } },
              headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('person_medicine_modal')
        expect(response.body).to include("person_medicine_#{person_medicine.id}")
      end

      it 'does not update medicine_id even if passed in params' do
        original_medicine_id = person_medicine.medicine_id
        patch person_person_medicine_path(person, person_medicine),
              params: { person_medicine: { medicine_id: medicines(:vitamin_c).id, notes: 'Updated' } }

        expect(response).to redirect_to(person_path(person))
        expect(person_medicine.reload.medicine_id).to eq(original_medicine_id)
        expect(person_medicine.reload.notes).to eq('Updated')
      end
    end

    context 'when signed in as parent of linked child' do
      before { sign_in(users(:parent)) }

      it 'updates the person medicine' do
        patch person_person_medicine_path(person, person_medicine),
              params: { person_medicine: { notes: 'Parent updated' } }

        expect(response).to redirect_to(person_path(person))
        expect(person_medicine.reload.notes).to eq('Parent updated')
      end
    end

    context 'when signed in as carer' do
      before { sign_in(users(:carer)) }

      it 'redirects (not authorized)' do
        carer_medicine = PersonMedicine.create!(person: assigned_patient, medicine: medicines(:vitamin_d))
        patch person_person_medicine_path(assigned_patient, carer_medicine),
              params: { person_medicine: { notes: 'Unauthorized update' } }

        expect(response).to redirect_to(root_path)
        expect(carer_medicine.reload.notes).to be_nil
      end
    end
  end
end
