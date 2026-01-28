# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin invites users' do
  fixtures :accounts, :account_otp_keys, :people, :users

  let(:admin) { users(:admin) }

  before do
    driven_by(:playwright)
    ActionMailer::Base.deliveries.clear
  end

  it 'allows an admin to send a user invitation email' do
    login_as(admin)

    visit admin_root_path
    click_link 'Invitations'

    fill_in 'Email', with: 'invited_parent@example.com'
    select 'Parent', from: 'Role'

    click_button 'Send invitation'

    expect(page).to have_content('Invitation sent')
    expect(ActionMailer::Base.deliveries.count).to eq(1)

    email = ActionMailer::Base.deliveries.last
    expect(email.to).to eq(['invited_parent@example.com'])
  end

  it 'allows an invitee to accept an invitation' do
    login_as(admin)

    visit admin_root_path
    click_link 'Invitations'

    fill_in 'Email', with: 'invited_parent@example.com'
    select 'Parent', from: 'Role'

    click_button 'Send invitation'

    email = ActionMailer::Base.deliveries.last
    expect(email).to be_present

    invitation_url = email.body.encoded.match(%r{https?://\S+})[0]
    uri = URI.parse(invitation_url)
    visit [uri.path, uri.query].compact.join('?')

    fill_in 'Name', with: 'Invited Parent'
    fill_in 'Date of birth', with: '1985-05-15'
    fill_in 'Password', with: 'securepassword123'
    fill_in 'Confirm Password', with: 'securepassword123'

    click_button 'Create Account'

    expect(page).to have_current_path('/dashboard')

    # Wait for flash message to auto-dismiss and be removed from DOM
    using_wait_time(5) do
      expect(page).to have_no_css('[data-controller="flash"]')
    end

    click_button 'Invited Parent'
    expect(page).to have_content('Profile')
  end
end
