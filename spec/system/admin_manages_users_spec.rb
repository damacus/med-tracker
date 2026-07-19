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
    person = Person.create!(
      household: admin.person.household,
      name: 'Carer User',
      date_of_birth: '1990-01-01',
      account: account
    )
    User.create!(person: person, email_address: 'test_carer@example.com',
                 password: 'password', password_confirmation: 'password')
  end
  let(:unverified_user) do
    account = Account.create!(email: 'unverified_user@example.com',
                              password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
                              status: :unverified)
    ActiveRecord::Base.connection.execute(
      "INSERT INTO account_verification_keys (account_id, key) VALUES (#{account.id}, 'manual-verify-key')"
    )
    person = Person.create!(
      household: admin.person.household,
      name: 'Unverified User',
      date_of_birth: '1992-02-02',
      account: account
    )
    User.create!(person: person, email_address: 'unverified_user@example.com',
                 password: 'password', password_confirmation: 'password')
  end

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
    attach_user_to_admin_household(carer)
    attach_user_to_admin_household(unverified_user)
  end

  it 'creates a user who can immediately log in', :js do
    login_as(admin)
    visit new_admin_user_path

    fill_in 'Name', with: 'Loginable User'
    fill_in 'Date of birth', with: '1985-05-15'
    fill_in 'Email address', with: 'loginable@example.com'
    fill_in 'user_password', with: 'SecureP@ssword123!'
    fill_in 'user_password_confirmation', with: 'SecureP@ssword123!'

    click_on 'Create User'
    expect(page).to have_text('User was successfully created')

    using_wait_time(5) do
      expect(page).to have_no_text('User was successfully created')
    end

    click_on 'Sign Out'
    visit login_path
    fill_in 'email', with: 'loginable@example.com'
    fill_in 'password', with: 'SecureP@ssword123!'
    click_on 'Sign In'

    expect(page).to have_text('Loginable User')
  end

  it 'edits, deactivates, and reactivates a user', :js do
    login_as(admin)
    visit admin_users_path

    within "[data-user-id='#{carer.id}']" do
      click_on 'Edit'
    end

    fill_in 'Email address', with: 'updated_carer@example.com'
    click_on 'Update User'

    expect(page).to have_text('User was successfully updated')
    expect(page).to have_text('updated_carer@example.com')

    within "[data-user-id='#{carer.id}']" do
      click_on 'Edit'
    end

    find_by_id('membership_role_trigger').click
    all('label', text: 'Administrator', visible: :all).last.click
    click_on 'Update household role'

    expect(page).to have_text('Membership role updated')
    within "[data-user-id='#{carer.id}']" do
      expect(page).to have_text('Administrator')
      click_button 'Deactivate'
    end

    within('[role="alertdialog"]') do
      click_button 'Deactivate'
    end

    within "[data-user-id='#{carer.id}']" do
      expect(page).to have_text('Inactive')
      click_button 'Activate'
    end

    within "[data-user-id='#{carer.id}']" do
      expect(page).to have_text('Active')
    end
  end

  it 'searches and clears the user list', :js do
    login_as(admin)
    visit admin_users_path

    fill_in 'Search', with: 'Carer'
    click_button 'Search'
    within '[data-testid="admin-users"]' do
      expect(page).to have_text('Carer User')
      expect(page).to have_no_text(admin.name)
    end

    click_link 'Clear'

    expect(page).to have_current_path(admin_users_path)
    expect(page).to have_text('Carer User')
    expect(page).to have_text(admin.name)
  end

  it 'filters the user list by membership role', :js do
    login_as(admin)
    visit admin_users_path

    find_by_id('role_trigger').click
    all('label', text: 'Member', visible: :all).last.click

    within '[data-testid="admin-users"]' do
      expect(page).to have_text('Carer User')
      expect(page).to have_no_text(admin.name)
    end
  end

  it 'manually verifies an unverified user and removes verification keys' do
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

  it 'denies a non-admin access to user management' do
    login_as(carer)

    visit admin_users_path

    expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
  end

  def attach_user_to_admin_household(user)
    household = ensure_api_household_for(admin)
    membership = household.household_memberships.find_or_create_by!(account: user.person.account) do |membership|
      membership.person = user.person
    end
    membership.update!(person: user.person, role: :member, status: :active)
  end
end
