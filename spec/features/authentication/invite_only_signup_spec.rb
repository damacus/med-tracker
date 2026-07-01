# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Invite-only sign-up', type: :system do
  fixtures :accounts, :people, :users

  describe 'when a household owner exists' do
    before { AppSettings.instance.update!(invite_only: true) }

    it 'redirects to login when visiting create-account without an invitation token' do
      visit create_account_path

      expect(page).to have_current_path(login_path)
      expect(page).to have_text(/invitation|invite/i)
    end

    it 'does not show the Create Account link on the login page' do
      visit login_path

      expect(page).to have_text('Welcome back')
      expect(page).to have_no_link('Create a New Account')
    end

    it 'allows visiting create-account with a valid invitation token' do
      invitation = create(:household_invitation, email: 'newuser@example.org', membership_role: :member)

      visit "#{create_account_path}?invitation_token=#{invitation.token}"

      expect(page).to have_current_path(%r{/create-account})
    end
  end

  describe 'when no household owner exists' do
    before do
      AppSettings.instance.update!(invite_only: false)
      HouseholdMembership.owner.find_each(&:destroy!)
    end

    it 'allows visiting create-account without an invitation token' do
      visit create_account_path

      expect(page).to have_current_path(%r{/create-account})
    end

    it 'shows the Create Account link on the login page' do
      visit login_path

      expect(page).to have_link('Create one')
    end
  end
end
