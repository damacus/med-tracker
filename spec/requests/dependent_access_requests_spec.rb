# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dependent access requests' do
  fixtures :accounts, :people, :users, :locations, :carer_relationships

  describe 'POST /people/:person_id/dependent_access_requests' do
    it 'allows a parent to request access without creating a carer relationship' do
      parent = users(:parent)
      dependent = people(:child_patient)
      sign_in(parent)

      expect do
        post person_dependent_access_requests_path(dependent)
      end.to change(DependentAccessRequest, :count).by(1)
        .and change(CarerRelationship, :count).by(0)

      request_record = DependentAccessRequest.last
      expect(request_record).to be_pending
      expect(request_record.requester).to eq(parent)
      expect(request_record.carer).to eq(parent.person)
      expect(request_record.patient).to eq(dependent)
    end
  end

  describe 'POST /admin/dependent_access_requests/:id/approve' do
    it 'allows an administrator to approve a parent request and create the relationship' do
      parent = users(:parent)
      dependent = people(:child_patient)
      request_record = DependentAccessRequest.create!(
        requester: parent,
        carer: parent.person,
        patient: dependent,
        relationship_type: 'parent'
      )
      sign_in(users(:admin))

      expect do
        post approve_admin_dependent_access_request_path(request_record)
      end.to change(CarerRelationship, :count).by(1)

      relationship = CarerRelationship.find_by!(carer: parent.person, patient: dependent)
      expect(relationship.relationship_type).to eq('parent')
      expect(relationship.active).to be true
      expect(request_record.reload).to be_approved
      expect(request_record.reviewer).to eq(users(:admin))
      expect(request_record.reviewed_at).to be_present
    end

    it 'does not allow a parent to approve their own request' do
      parent = users(:parent)
      dependent = people(:child_patient)
      request_record = DependentAccessRequest.create!(
        requester: parent,
        carer: parent.person,
        patient: dependent,
        relationship_type: 'parent'
      )
      sign_in(parent)

      expect do
        post approve_admin_dependent_access_request_path(request_record)
      end.not_to change(CarerRelationship, :count)

      expect(request_record.reload).to be_pending
    end
  end

  describe 'POST /admin/dependent_access_requests/:id/reject' do
    it 'allows an administrator to reject a parent request without creating a relationship' do
      parent = users(:parent)
      dependent = people(:child_patient)
      request_record = DependentAccessRequest.create!(
        requester: parent,
        carer: parent.person,
        patient: dependent,
        relationship_type: 'parent'
      )
      sign_in(users(:admin))

      expect do
        post reject_admin_dependent_access_request_path(request_record)
      end.not_to change(CarerRelationship, :count)

      expect(request_record.reload).to be_rejected
      expect(request_record.reviewer).to eq(users(:admin))
      expect(request_record.reviewed_at).to be_present
    end
  end
end
