# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Clinician Access Authorization' do
  fixtures :users, :people

  before do
    driven_by(:playwright)
  end

  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }

  describe 'viewing people' do
    it 'allows doctors to view all people' do
      sign_in_as(doctor)
      visit people_path

      expect(page).to have_content('People')
      expect(Person.count).to be > 0
    end

    it 'allows nurses to view all people' do
      sign_in_as(nurse)
      visit people_path

      expect(page).to have_content('People')
      expect(Person.count).to be > 0
    end

    it 'allows doctors to view any person' do
      sign_in_as(doctor)
      person = people(:john)
      visit person_path(person)

      expect(page).to have_content(person.name)
    end
  end

  describe 'managing people' do
    it 'denies doctors ability to create new people' do
      sign_in_as(doctor)
      visit people_path

      expect(page).to have_css('h1', text: 'People')
      expect(page).to have_no_link('New Person')
    end

    it 'denies nurses ability to edit people' do
      sign_in_as(nurse)
      person = people(:john)
      visit person_path(person)

      expect(page).to have_content(person.name)
      expect(page).to have_no_link('Edit')
    end

    it 'denies doctors ability to delete people' do
      sign_in_as(doctor)
      person = people(:john)
      visit person_path(person)

      expect(page).to have_content(person.name)
      expect(page).to have_no_button('Delete')
    end
  end

  describe 'user management' do
    it 'denies doctors access to user management' do
      sign_in_as(doctor)
      visit admin_users_path

      expect(page).to have_content('You are not authorized to perform this action')
    end

    it 'denies nurses access to user management' do
      sign_in_as(nurse)
      visit admin_users_path

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  def sign_in_as(user, password: 'password')
    login_as(user)
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: password
    click_button 'Login'
    expect(page).to have_current_path(dashboard_path)
  end
end
