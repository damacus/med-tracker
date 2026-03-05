# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::CarerRelationships management' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }
  let(:non_admin) { users(:carer) }

  describe 'as an administrator' do
    before do
      sign_in(admin)
    end

    describe 'GET /admin/carer_relationships' do
      it 'returns success' do
        get admin_carer_relationships_path

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('Carer Relationships')
      end
    end

    describe 'POST /admin/carer_relationships' do
      it 'creates a new relationship' do
        expect do
          post admin_carer_relationships_path, params: {
            carer_relationship: {
              carer_id: people(:jane).id,
              patient_id: people(:john).id,
              relationship_type: 'family_member'
            }
          }
        end.to change(CarerRelationship, :count).by(1)

        expect(response).to redirect_to(admin_carer_relationships_path)
      end
    end

    describe 'DELETE /admin/carer_relationships/:id' do
      it 'deactivates the relationship' do
        relationship = carer_relationships(:jane_cares_for_child)

        delete admin_carer_relationship_path(relationship)

        expect(response).to redirect_to(admin_carer_relationships_path)
        expect(relationship.reload).not_to be_active
      end
    end

    describe 'POST /admin/carer_relationships/:id/activate' do
      it 'activates the relationship' do
        relationship = carer_relationships(:inactive_relationship)

        post activate_admin_carer_relationship_path(relationship)

        expect(response).to redirect_to(admin_carer_relationships_path)
        expect(relationship.reload).to be_active
      end
    end
  end

  describe 'as a non-administrator' do
    before do
      sign_in(non_admin)
    end

    it 'denies index access' do
      get admin_carer_relationships_path

      expect(response).to redirect_to(root_path)
    end

    it 'denies create access' do
      post admin_carer_relationships_path, params: {
        carer_relationship: {
          carer_id: people(:jane).id,
          patient_id: people(:john).id,
          relationship_type: 'family_member'
        }
      }

      expect(response).to redirect_to(root_path)
    end
  end
end
