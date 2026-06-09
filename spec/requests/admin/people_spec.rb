# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::People' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  describe 'GET /admin/people' do
    context 'as an administrator' do
      before { sign_in(admin) }

      it 'returns success' do
        get admin_people_path

        expect(response).to have_http_status(:success)
      end

      it 'lists a patient who needs a carer' do
        patient = create(:person, name: 'Needs A Carer')
        patient.update_column(:has_capacity, false)

        get admin_people_path

        expect(response.body).to include('Needs A Carer')
      end
    end

    context 'as a non-administrator' do
      before { sign_in(regular_user) }

      it 'denies access' do
        get admin_people_path

        expect(response).to redirect_to(root_path)
      end
    end

    context 'when unauthenticated' do
      it 'redirects to login' do
        get admin_people_path

        expect(response).to redirect_to(login_path)
      end
    end
  end
end
