# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin invites users' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
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

  it 'does not offer Minor in the invitation role selector' do
    login_as(admin)

    visit admin_root_path
    click_link 'Invitations'

    expect(page).to have_no_select('Role', with_options: ['Minor'])
  end

  it 'allows an invitee to accept an invitation' do
    login_as(admin)

    visit admin_root_path
    click_link 'Invitations'

    fill_in 'Email', with: 'invited_parent@example.com'
    select 'Parent', from: 'Role'

    click_button 'Send invitation'

    expect(page).to have_content('Invitation sent')

    email = ActionMailer::Base.deliveries.last
    expect(email).to be_present

    invitation_url = email.body.encoded.match(%r{https?://\S+})[0]
    uri = URI.parse(invitation_url)
    visit [uri.path, uri.query].compact.join('?')

    fill_in 'Name', with: 'Invited Parent'
    fill_in 'Date of birth', with: '1985-05-15'
    fill_in 'Password', with: 'SecureP@ssword123!'
    fill_in 'Confirm Password', with: 'SecureP@ssword123!'

    click_button 'Create Account'

    expect(page).to have_current_path('/dashboard')

    # Verify sidebar shows new user info
    within 'aside' do
      expect(page).to have_content('Invited Parent')
      expect(page).to have_content('Parent')
    end
  end

  it 'rejects under-18 invited signup' do
    login_as(admin)

    visit admin_root_path
    click_link 'Invitations'

    fill_in 'Email', with: 'invited_child@example.com'
    select 'Parent', from: 'Role'
    click_button 'Send invitation'

    invitation_url = ActionMailer::Base.deliveries.last.body.encoded.match(%r{https?://\S+})[0]
    uri = URI.parse(invitation_url)
    visit [uri.path, uri.query].compact.join('?')

    fill_in 'Name', with: 'Invited Child'
    fill_in 'Date of birth', with: 10.years.ago.to_date.to_s
    fill_in 'Password', with: 'SecureP@ssword123!'
    fill_in 'Confirm Password', with: 'SecureP@ssword123!'

    expect do
      click_button 'Create Account'
    end.not_to change(Account, :count)

    expect(page).to have_content('Children must be added by a parent or carer.')
  end

  it 'allows an admin to resend an invitation and invalidates the old token' do
    login_as(admin)
    invitation = create(:invitation, email: 'resend.me@example.com', role: :parent, expires_at: 1.day.ago)
    original_token = invitation.token

    visit admin_invitations_path

    within '#admin_invitations' do
      expect(page).to have_content('resend.me@example.com')
      click_button 'Resend', match: :first
    end

    expect(page).to have_content('Invitation resent')
    expect(ActionMailer::Base.deliveries.count).to eq(1)

    invitation.reload
    expect(invitation.token).not_to eq(original_token)

    visit accept_invitation_path(token: original_token)
    expect(page).to have_content('This invitation link is invalid or has expired.')

    visit accept_invitation_path(token: invitation.token)
    expect(page).to have_content("You've been invited as a Parent.")
  end
end
