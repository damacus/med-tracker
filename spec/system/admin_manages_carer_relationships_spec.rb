# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesCarerRelationships' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }
  let(:carer) { users(:carer) }
  let(:jane) { people(:jane) }

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

    it 'allows carer to access patient after relationship is reactivated' do
      relationship = carer_relationships(:inactive_relationship)
      carer_user = relationship.carer.user
      patient = relationship.patient

      login_as(carer_user)
      visit person_path(patient)
      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')

      rodauth_logout
      login_as(admin)
      visit admin_carer_relationships_path

      within "[data-relationship-id='#{relationship.id}']" do
        click_button 'Activate'
      end

      expect(page).to have_content('Carer relationship has been activated')

      rodauth_logout
      login_as(carer_user)
      visit person_path(patient)
      expect(page).to have_content(patient.name)
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
