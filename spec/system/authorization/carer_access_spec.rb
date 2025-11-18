# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Carer Access Authorization' do
  fixtures :users, :people, :carer_relationships

  before do
    driven_by(:playwright)
  end

  let(:carer) { users(:carer) }
  let(:assigned_patient) { people(:child_patient) }
  let(:unrelated_person) { people(:john) }

  describe 'viewing people' do
    it 'allows carers to view their own profile' do
      sign_in_as(carer)
      visit person_path(carer.person)

      expect(page).to have_content(carer.person.name)
    end

    it 'allows carers to view assigned patients' do
      sign_in_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content(assigned_patient.name)
    end

    it 'denies carers access to unrelated people' do
      sign_in_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  describe 'managing people' do
    it 'denies carers ability to create new people' do
      sign_in_as(carer)
      visit people_path

      expect(page).to have_css('h1', text: 'People')
      expect(page).to have_no_link('New Person')
    end

    it 'denies carers ability to edit people' do
      sign_in_as(carer)
      visit person_path(carer.person)

      expect(page).to have_content(carer.person.name)
      expect(page).to have_no_link('Edit')
    end

    it 'denies carers ability to delete people' do
      sign_in_as(carer)
      visit person_path(carer.person)

      expect(page).to have_content(carer.person.name)
      expect(page).to have_no_button('Delete')
    end
  end

  describe 'people index' do
    it 'shows only accessible people to carers' do
      sign_in_as(carer)
      visit people_path

      expect(page).to have_content(carer.person.name)
      expect(page).to have_content(assigned_patient.name)
      expect(page).to have_no_content(unrelated_person.name)
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
