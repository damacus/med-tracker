# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medication edit and update' do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships

  let(:person) { people(:child_user_person) }
  let(:assigned_patient) { people(:child_patient) }

  let!(:person_medication) do
    PersonMedication.create!(
      person: person,
      medication: medications(:vitamin_d),
      notes: 'Original notes',
      max_daily_doses: 3,
      min_hours_between_doses: 4
    )
  end

  describe 'GET /people/:person_id/person_medications/:id/edit' do
    context 'when signed in as admin' do
      before { sign_in(users(:admin)) }

      it 'returns 200' do
        get edit_person_person_medication_path(person, person_medication)
        expect(response).to have_http_status(:ok)
      end
    end

    context 'when signed in as parent of linked child' do
      before { sign_in(users(:parent)) }

      it 'returns 200' do
        get edit_person_person_medication_path(person, person_medication)
        expect(response).to have_http_status(:ok)
      end
    end

    context 'when signed in as carer' do
      before { sign_in(users(:carer)) }

      it 'redirects (not authorized)' do
        carer_medication = PersonMedication.create!(person: assigned_patient, medication: medications(:vitamin_d))
        get edit_person_person_medication_path(assigned_patient, carer_medication)
        expect(response).to redirect_to(root_path)
      end
    end
  end

  describe 'PATCH /people/:person_id/person_medications/:id' do
    context 'when signed in as admin' do
      before { sign_in(users(:admin)) }

      it 'updates the person medication and redirects' do
        patch person_person_medication_path(person, person_medication),
              params: { person_medication: { notes: 'Updated notes', max_daily_doses: 5 } }

        expect(response).to redirect_to(person_path(person))
        person_medication.reload
        expect(person_medication.notes).to eq('Updated notes')
        expect(person_medication.max_daily_doses).to eq(5)
      end

      it 'updates via Turbo Stream' do
        patch person_person_medication_path(person, person_medication),
              params: { person_medication: { notes: 'Turbo update' } },
              headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('modal')
        expect(response.body).to include("person_medication_#{person_medication.id}")
      end

      it 'does not update medication_id even if passed in params' do
        original_medication_id = person_medication.medication_id
        patch person_person_medication_path(person, person_medication),
              params: { person_medication: { medication_id: medications(:vitamin_c).id, notes: 'Updated' } }

        expect(response).to redirect_to(person_path(person))
        expect(person_medication.reload.medication_id).to eq(original_medication_id)
        expect(person_medication.reload.notes).to eq('Updated')
      end
    end

    context 'when signed in as parent of linked child' do
      before { sign_in(users(:parent)) }

      it 'updates the person medication' do
        patch person_person_medication_path(person, person_medication),
              params: { person_medication: { notes: 'Parent updated' } }

        expect(response).to redirect_to(person_path(person))
        expect(person_medication.reload.notes).to eq('Parent updated')
      end
    end

    context 'when signed in as carer' do
      before { sign_in(users(:carer)) }

      it 'redirects (not authorized)' do
        carer_medication = PersonMedication.create!(person: assigned_patient, medication: medications(:vitamin_d))
        patch person_person_medication_path(assigned_patient, carer_medication),
              params: { person_medication: { notes: 'Unauthorized update' } }

        expect(response).to redirect_to(root_path)
        expect(carer_medication.reload.notes).to be_nil
      end
    end
  end
end
