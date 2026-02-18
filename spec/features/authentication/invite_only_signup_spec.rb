# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Invite-only sign-up', type: :system do
  fixtures :accounts, :people, :users

  describe 'when an administrator exists' do
    it 'redirects to login when visiting create-account without an invitation token' do
      visit create_account_path

      expect(page).to have_current_path(login_path)
      expect(page).to have_content(/invitation|invite/i)
    end

    it 'does not show the Create Account link on the login page' do
      visit login_path

      expect(page).to have_content('Welcome back')
      expect(page).to have_no_link('Create a New Account')
    end

    it 'allows visiting create-account with a valid invitation token' do
      invitation = Invitation.create!(email: 'newuser@example.org', role: :nurse)

      visit "#{create_account_path}?invitation_token=#{invitation.token}"

      expect(page).to have_current_path(%r{/create-account})
    end
  end

  describe 'when no administrator exists' do
    before { User.administrator.delete_all }

    it 'allows visiting create-account without an invitation token' do
      visit create_account_path

      expect(page).to have_current_path(%r{/create-account})
    end

    it 'shows the Create Account link on the login page' do
      visit login_path

      expect(page).to have_link('Create a New Account')
    end
  end
end
