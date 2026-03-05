# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::Users management' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:admin) { users(:admin) }
  let(:non_admin) { users(:carer) }

  describe 'as an administrator' do
    before do
      sign_in(admin)
    end

    describe 'GET /admin/users' do
      it 'returns success with search, filter, and sort params' do
        get admin_users_path, params: {
          search: 'Jane',
          role: 'parent',
          status: 'active',
          sort: 'name',
          direction: 'asc'
        }

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('User Management')
      end
    end

    describe 'POST /admin/users' do
      it 'creates a new user and account' do
        expect do
          post admin_users_path, params: {
            user: {
              email_address: 'request_new_user@example.com',
              password: 'password',
              password_confirmation: 'password',
              role: 'carer',
              person_attributes: {
                name: 'Request New User',
                date_of_birth: '1990-01-01',
                location_ids: [locations(:home).id]
              }
            }
          }
        end.to change(User, :count).by(1).and change(Account, :count).by(1)

        expect(response).to redirect_to(admin_users_path)
      end

      it 'returns unprocessable content for duplicate email' do
        post admin_users_path, params: {
          user: {
            email_address: users(:jane).email_address,
            password: 'password',
            password_confirmation: 'password',
            role: 'carer',
            person_attributes: {
              name: 'Duplicate User',
              date_of_birth: '1990-01-01',
              location_ids: [locations(:home).id]
            }
          }
        }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('has already been taken')
      end
    end

    describe 'PATCH /admin/users/:id' do
      it 'updates the user and redirects to index' do
        user = users(:jane)

        patch admin_user_path(user), params: {
          user: {
            email_address: user.email_address,
            role: 'doctor',
            person_attributes: {
              id: user.person.id,
              name: 'Jane Updated',
              date_of_birth: user.person.date_of_birth.to_s,
              location_ids: [locations(:home).id]
            }
          }
        }

        expect(response).to redirect_to(admin_users_path)
        expect(user.reload.role).to eq('doctor')
        expect(user.person.reload.name).to eq('Jane Updated')
      end
    end

    describe 'POST /admin/users/:id/activate' do
      it 'activates a deactivated user' do
        user = users(:carer)
        user.deactivate!

        post activate_admin_user_path(user)

        expect(response).to redirect_to(admin_users_path)
        expect(user.reload).to be_active
      end
    end

    describe 'POST /admin/users/:id/verify' do
      it 'verifies an unverified account and removes verification keys' do
        user = users(:jane)
        account = user.person.account
        account.update!(status: :unverified)
        ActiveRecord::Base.connection.execute(
          "INSERT INTO account_verification_keys (account_id, key) VALUES (#{account.id}, 'request-verify-key')"
        )

        post verify_admin_user_path(user)

        expect(response).to redirect_to(admin_users_path)
        expect(account.reload).to be_verified
        key_count = ActiveRecord::Base.connection.select_value(
          "SELECT COUNT(*) FROM account_verification_keys WHERE account_id = #{account.id}"
        ).to_i
        expect(key_count).to eq(0)
      end
    end

    describe 'DELETE /admin/users/:id' do
      it 'does not allow administrators to deactivate themselves' do
        delete admin_user_path(admin)

        expect(response).to redirect_to(admin_users_path)
        expect(admin.reload).to be_active
      end
    end
  end

  describe 'as a non-administrator' do
    before do
      sign_in(non_admin)
    end

    it 'denies index access' do
      get admin_users_path

      expect(response).to redirect_to(root_path)
    end

    it 'denies new access' do
      get new_admin_user_path

      expect(response).to redirect_to(root_path)
    end
  end
end
