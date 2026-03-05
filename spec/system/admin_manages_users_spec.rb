# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesUsers' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  # Use the admin fixture instead of creating a duplicate user
  let(:admin) { users(:admin) }
  # Use a unique email for the carer user to avoid conflicts
  let!(:carer) do
    account = Account.create!(email: 'test_carer@example.com',
                              password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
                              status: 'verified')
    person = Person.create!(name: 'Carer User', date_of_birth: '1990-01-01', account: account)
    User.create!(person: person, email_address: 'test_carer@example.com',
                 password: 'password', password_confirmation: 'password', role: :carer)
  end
  let!(:unverified_user) do
    account = Account.create!(email: 'unverified_user@example.com',
                              password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
                              status: :unverified)
    ActiveRecord::Base.connection.execute(
      "INSERT INTO account_verification_keys (account_id, key) VALUES (#{account.id}, 'manual-verify-key')"
    )
    person = Person.create!(name: 'Unverified User', date_of_birth: '1992-02-02', account: account)
    User.create!(person: person, email_address: 'unverified_user@example.com',
                 password: 'password', password_confirmation: 'password', role: :parent)
  end

  before do
    driven_by(:playwright)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of users' do
      login_as(admin)

      visit admin_users_path

      within '[data-testid="admin-users"]' do
        expect(page).to have_content('User Management')
        expect(page).to have_content(admin.email_address)
        expect(page).to have_content(carer.email_address)
      end
    end

    it 'shows separate activation and verification status columns' do
      login_as(admin)

      visit admin_users_path

      expect(page).to have_css('th', text: 'Activation')
      expect(page).to have_css('th', text: 'Verification')
    end

    it 'allows admin to manually verify an unverified user and removes verification keys' do
      login_as(admin)

      visit admin_users_path

      within "[data-user-id='#{unverified_user.id}']" do
        click_button 'Verify'
      end

      expect(unverified_user.person.account.reload).to be_verified
      key_count = ActiveRecord::Base.connection.select_value(
        "SELECT COUNT(*) FROM account_verification_keys WHERE account_id = #{unverified_user.person.account.id}"
      ).to_i
      expect(key_count).to eq(0)

      within "[data-user-id='#{unverified_user.id}']" do
        expect(page).to have_button('Verified', disabled: true)
      end
    end

    it 'prevents admin from deactivating themselves' do
      login_as(admin)

      visit admin_users_path

      within "[data-user-id='#{admin.id}']" do
        expect(page).to have_no_button('Deactivate')
      end
    end

    it 'shows pagination info' do
      login_as(admin)

      visit admin_users_path

      # Pagination info is hidden on mobile, check for visible on desktop
      expect(page).to have_css('[data-testid="pagination-info"]', visible: :all)
    end
  end

  context 'when user is logged in as a non-admin' do
    it 'denies access to the user list' do
      login_as(carer)

      visit admin_users_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end

    it 'denies access to create new users' do
      login_as(carer)

      visit new_admin_user_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end
  end
end
