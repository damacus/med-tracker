# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medicine reordering' do
  fixtures :accounts, :people, :users, :locations, :medicines, :carer_relationships

  let(:parent_user) { users(:parent) }
  let(:linked_child) { people(:child_user_person) }
  let(:unlinked_child) { people(:child_patient) }

  before do
    sign_in(parent_user)
  end

  describe 'PATCH /people/:person_id/person_medicines/:id/reorder' do
    it 'allows parents to reorder medicines for linked children and persists ordering' do
      linked_first = PersonMedicine.create!(person: linked_child, medicine: medicines(:vitamin_d))
      linked_second = PersonMedicine.create!(person: linked_child, medicine: medicines(:vitamin_c))

      patch reorder_person_person_medicine_path(linked_child, linked_second), params: { direction: 'up' }

      expect(response).to redirect_to(person_path(linked_child))
      expect(linked_child.person_medicines.order(:position, :id).pluck(:id)).to eq([linked_second.id, linked_first.id])
    end

    it 'denies parents from reordering medicines for unlinked children' do
      unlinked_first = PersonMedicine.create!(person: unlinked_child, medicine: medicines(:ibuprofen))
      unlinked_second = PersonMedicine.create!(person: unlinked_child, medicine: medicines(:aspirin))
      original_order = unlinked_child.person_medicines.order(:position, :id).pluck(:id)

      patch reorder_person_person_medicine_path(unlinked_child, unlinked_second), params: { direction: 'up' }

      expect(response).to redirect_to(root_path)
      expect(unlinked_child.person_medicines.order(:position, :id).pluck(:id))
        .to eq([unlinked_first.id, unlinked_second.id])
      expect(unlinked_child.person_medicines.order(:position, :id).pluck(:id)).to eq(original_order)
    end
  end
end
