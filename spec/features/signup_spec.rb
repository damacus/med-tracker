# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'User Signup', type: :system do
  describe 'creating a new account' do
    it 'creates both an Account and a Person with valid details' do
      visit create_account_path

      fill_in 'Name', with: 'New Test User'
      fill_in 'Date of birth', with: '1990-01-15'
      fill_in 'Email', with: 'newuser@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      expect do
        click_button 'Create Account'
      end.to change(Account, :count).by(1)
                                    .and change(Person, :count).by(1)

      # Should redirect to dashboard or verification page
      expect(page).to have_current_path('/dashboard').or have_current_path('/verify-account-resend')

      # Verify the account was created with correct email
      account = Account.find_by(email: 'newuser@example.com')
      expect(account).to be_present

      # Verify the person was created and linked to the account
      person = account.person
      expect(person).to be_present
      expect(person.name).to eq('New Test User')
      expect(person.date_of_birth).to eq(Date.new(1990, 1, 15))
      expect(person.email).to eq('newuser@example.com')
      expect(person.person_type).to eq('adult')
    end

    it 'shows validation errors when name is missing' do
      visit create_account_path

      fill_in 'Date of birth', with: '1990-01-15'
      fill_in 'Email', with: 'newuser@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      expect do
        click_button 'Create Account'
      end.not_to change(Account, :count)

      expect(page).to have_content('Name')
    end

    it 'shows validation errors when date of birth is missing' do
      visit create_account_path

      fill_in 'Name', with: 'New Test User'
      fill_in 'Email', with: 'newuser@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      expect do
        click_button 'Create Account'
      end.not_to change(Account, :count)

      expect(page).to have_content('Date of birth')
    end

    it 'shows validation errors when passwords do not match' do
      visit create_account_path

      fill_in 'Name', with: 'New Test User'
      fill_in 'Date of birth', with: '1990-01-15'
      fill_in 'Email', with: 'newuser@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'differentpassword'

      expect do
        click_button 'Create Account'
      end.not_to change(Account, :count)

      expect(page).to have_content(/password/i)
    end

    it 'shows validation errors when email is already taken' do
      # Create an existing account
      Account.create!(
        email: 'existing@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
        status: 'verified'
      )

      visit create_account_path

      fill_in 'Name', with: 'New Test User'
      fill_in 'Date of birth', with: '1990-01-15'
      fill_in 'Email', with: 'existing@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      expect do
        click_button 'Create Account'
      end.not_to change(Account, :count)

      expect(page).to have_content(/already.*taken|already.*account/i)
    end

    it 'sets person_type based on age (adult for 18+)' do
      visit create_account_path

      # Set date of birth to make user 30 years old
      fill_in 'Name', with: 'Adult User'
      fill_in 'Date of birth', with: 30.years.ago.to_date.to_s
      fill_in 'Email', with: 'adult@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      click_button 'Create Account'

      person = Account.find_by(email: 'adult@example.com')&.person
      expect(person&.person_type).to eq('adult')
    end

    it 'sets person_type based on age (minor for under 18)' do
      visit create_account_path

      # Set date of birth to make user 15 years old
      fill_in 'Name', with: 'Minor User'
      fill_in 'Date of birth', with: 15.years.ago.to_date.to_s
      fill_in 'Email', with: 'minor@example.com'
      fill_in 'Password', with: 'securepassword123'
      fill_in 'Confirm Password', with: 'securepassword123'

      click_button 'Create Account'

      person = Account.find_by(email: 'minor@example.com')&.person
      expect(person&.person_type).to eq('minor')
    end
  end
end
