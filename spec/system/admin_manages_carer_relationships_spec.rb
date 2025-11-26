# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesCarerRelationships' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }
  let(:carer) { users(:carer) }
  let(:jane) { people(:jane) }
  let(:child_patient) { people(:child_patient) }

  before do
    driven_by(:playwright)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of carer relationships' do
      login_as(admin)

      visit admin_carer_relationships_path

      expect(page).to have_content('Carer Relationships')
      expect(page).to have_content(jane.name)
    end

    it 'allows admin to create a new carer relationship' do
      login_as(admin)

      visit admin_carer_relationships_path
      click_link 'New Relationship'

      expect(page).to have_content('New Carer Relationship')

      select jane.name, from: 'Carer'
      select people(:john).name, from: 'Patient'
      select 'Family member', from: 'Relationship type'

      click_button 'Create Relationship'

      expect(page).to have_content('Carer relationship was successfully created')
    end

    it 'allows admin to deactivate a carer relationship' do
      relationship = carer_relationships(:jane_cares_for_child)
      login_as(admin)

      visit admin_carer_relationships_path

      within "[data-relationship-id='#{relationship.id}']" do
        click_button 'Deactivate'
      end

      # Confirm in the AlertDialog
      within('[role="alertdialog"]') do
        click_button 'Deactivate'
      end

      expect(page).to have_content('Carer relationship has been deactivated')
    end
  end

  context 'when user is logged in as a non-admin' do
    it 'denies access to the carer relationships list' do
      login_as(carer)

      visit admin_carer_relationships_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end
  end
end
