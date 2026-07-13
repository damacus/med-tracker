# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesCarerRelationships' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }
  let(:carer) { users(:carer) }
  let(:jane) { people(:jane) }
  let(:child_patient) { people(:child_patient) }

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of carer relationships' do
      login_as(admin)

      visit admin_carer_relationships_path

      expect(page).to have_text('Carer Relationships')
      expect(page).to have_text(jane.name)
    end

    it 'allows admin to create a new carer relationship' do
      login_as(admin)

      visit admin_carer_relationships_path
      click_link 'New Relationship'

      expect(page).to have_text('New Carer Relationship')

      select jane.name, from: 'Carer'
      select people(:john).name, from: 'Patient'
      select 'Family member', from: 'Relationship type'

      click_button 'Create Relationship'

      expect(page).to have_text('Carer relationship was successfully created')
      relationship = CarerRelationship.find_by!(carer: jane, patient: people(:john))
      expect(relationship.person_access_grants.sole).to have_attributes(
        access_level: 'manage',
        relationship_type: 'family_member'
      )
    end

    it 'allows admin to deactivate a carer relationship', :js do
      login_as(admin)
      household = admin.person.household
      actor_membership = household.household_memberships.find_by!(account: admin.person.account)
      relationship = CareDelegation::Assign.new(
        carer: carer.person,
        patient: people(:john),
        relationship_type: :family_member,
        granted_by_membership: actor_membership
      ).call
      grant = relationship.person_access_grants.sole

      visit admin_carer_relationships_path

      within "[data-relationship-id='#{relationship.id}']" do
        click_button 'Deactivate'
      end

      # Confirm in the AlertDialog
      within('[role="alertdialog"]') do
        click_button 'Deactivate'
      end

      expect(page).to have_text('Carer relationship has been deactivated')
      expect(relationship.reload).not_to be_active
      expect(grant.reload.revoked_at).to be_present
    end

    it 'allows admin to reactivate a deactivated carer relationship' do
      relationship = carer_relationships(:inactive_relationship)
      login_as(admin)

      visit admin_carer_relationships_path

      within "[data-relationship-id='#{relationship.id}']" do
        expect(page).to have_text('Inactive')
        click_button 'Activate'
      end

      expect(page).to have_text('Carer relationship has been activated')
      expect(relationship.reload.person_access_grants.sole.carer_relationship).to eq(relationship)

      within "[data-relationship-id='#{relationship.id}']" do
        expect(page).to have_text('Active')
      end
    end

    it 'allows carer to access patient after relationship is reactivated' do
      relationship = carer_relationships(:inactive_relationship)
      carer_user = relationship.carer.user
      patient = relationship.patient

      expect(relationship).not_to be_active
      carer_memberships = carer_user.person.account.household_memberships
      grants = PersonAccessGrant.active.where(person: patient, household_membership: carer_memberships)
      expect(grants).to be_empty

      # Admin reactivates the relationship
      login_as(admin)
      visit admin_carer_relationships_path

      within "[data-relationship-id='#{relationship.id}']" do
        click_button 'Activate'
      end

      expect(page).to have_text('Carer relationship has been activated')

      # Now carer can access patient
      rodauth_logout
      login_as(carer_user)
      visit person_path(patient)
      expect(page).to have_text(patient.name)
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
