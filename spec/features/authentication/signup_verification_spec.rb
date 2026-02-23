# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Signup verification email', type: :system do
  fixtures :accounts, :people, :users

  let(:new_user_email) { 'newuser@example.org' }
  let(:new_user_password) { 'Password123!' }

  before do
    ActionMailer::Base.deliveries.clear
  end

  describe 'after account creation via invitation' do
    let!(:invitation) { Invitation.create!(email: new_user_email, role: :nurse) }

    before do
      visit accept_invitation_path(token: invitation.token)
      fill_in 'Name', with: 'New User'
      fill_in 'Date of birth', with: '1990-01-01'
      fill_in 'Password', with: new_user_password
      fill_in 'Confirm Password', with: new_user_password
      click_button 'Create Account'
    end

    it 'sends a verification email to the new user' do
      email = ActionMailer::Base.deliveries.find { |m| m.to.include?(new_user_email) }
      expect(email).to be_present
    end

    it 'sends the verification email from the configured from address' do
      email = ActionMailer::Base.deliveries.find { |m| m.to.include?(new_user_email) }
      expect(email.from).to include('noreply@medtracker.app')
    end

    it 'sends a verification email with a subject containing "Verify"' do
      email = ActionMailer::Base.deliveries.find { |m| m.to.include?(new_user_email) }
      expect(email.subject).to include('Verify')
    end

    it 'includes a verification link in the email body' do
      email = ActionMailer::Base.deliveries.find { |m| m.to.include?(new_user_email) }
      expect(email.body.encoded).to include('verify-account')
    end

    it 'allows the user to verify their account by visiting the link' do
      email = ActionMailer::Base.deliveries.find { |m| m.to.include?(new_user_email) }
      verify_url = email.body.encoded[%r{https?://\S+verify-account\S*}]
      uri = URI.parse(verify_url)
      visit [uri.path, uri.query].compact.join('?')

      expect(page).to have_content(/verified|dashboard/i)
    end
  end
end
