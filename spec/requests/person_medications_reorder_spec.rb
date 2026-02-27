# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medication reordering' do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships

  let(:parent_user) { users(:parent) }
  let(:linked_child) { people(:child_user_person) }
  let(:unlinked_child) { people(:child_patient) }

  before do
    sign_in(parent_user)
  end

  describe 'PATCH /people/:person_id/person_medications/:id/reorder' do
    it 'allows parents to reorder medications for linked children and persists ordering' do
      linked_first = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_d))
      linked_second = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_c))

      patch reorder_person_person_medication_path(linked_child, linked_second), params: { direction: 'up' }

      expect(response).to redirect_to(person_path(linked_child))
      expect(linked_child.person_medications.order(:position,
                                                   :id).pluck(:id)).to eq([linked_second.id, linked_first.id])
    end

    it 'denies parents from reordering medications for unlinked children' do
      unlinked_first = PersonMedication.create!(person: unlinked_child, medication: medications(:ibuprofen))
      unlinked_second = PersonMedication.create!(person: unlinked_child, medication: medications(:aspirin))
      original_order = unlinked_child.person_medications.order(:position, :id).pluck(:id)

      patch reorder_person_person_medication_path(unlinked_child, unlinked_second), params: { direction: 'up' }

      expect(response).to redirect_to(root_path)
      expect(unlinked_child.person_medications.order(:position, :id).pluck(:id))
        .to eq([unlinked_first.id, unlinked_second.id])
      expect(unlinked_child.person_medications.order(:position, :id).pluck(:id)).to eq(original_order)
    end

    it 'returns turbo_stream and updates person show container for linked children' do
      linked_first = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_d))
      linked_second = PersonMedication.create!(person: linked_child, medication: medications(:vitamin_c))

      patch reorder_person_person_medication_path(linked_child, linked_second),
            params: { direction: 'up' },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"person_show_#{linked_child.id}\"")
      expect(linked_child.person_medications.order(:position,
                                                   :id).pluck(:id)).to eq([linked_second.id, linked_first.id])
    end
  end
end
