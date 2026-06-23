# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People carer relationships' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  describe 'POST /people/:person_id/carer_relationships' do
    it 'allows an admin to assign an existing adult with a selected relationship type' do
      sign_in(users(:admin))

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: {
               carer_relationship: {
                 carer_id: users(:jane).person.id,
                 relationship_type: 'family_member'
               }
             }
      end.to change(CarerRelationship, :count).by(1)

      relationship = CarerRelationship.find_by!(carer: users(:jane).person, patient: people(:child_user_person))
      expect(relationship.relationship_type).to eq('family_member')
      expect(relationship.active).to be true
      expect(response).to redirect_to(person_path(people(:child_user_person)))
    end

    it 'renders an error when an admin submits without a carer' do
      sign_in(users(:admin))

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { carer_id: '', relationship_type: 'family_member' } }
      end.not_to change(CarerRelationship, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Select an adult user to assign')
    end

    it 'allows an existing parent to link another parent by email for their child' do
      sign_in(users(:parent))
      write_legacy_email(users(:jane), :email_address, 'Jane.Parent@example.com')

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: 'jane.parent@example.com' } }
      end.to change(CarerRelationship, :count).by(1)

      relationship = CarerRelationship.find_by!(carer: users(:jane).person, patient: people(:child_user_person))
      expect(relationship.relationship_type).to eq('parent')
      expect(relationship.active).to be true
      expect(response).to redirect_to(person_path(people(:child_user_person)))
    end

    it 'allows the assigned second parent to access the child' do
      sign_in(users(:parent))

      post person_carer_relationships_path(people(:child_user_person)),
           params: { carer_relationship: { email: users(:jane).email_address } }

      post '/logout'
      sign_in(users(:jane))
      get person_path(people(:child_user_person))

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:child_user_person).name)
    end

    it 'reactivates a revoked access grant when reassigning an existing parent' do
      sign_in(users(:parent))
      household = Household.find_by!(slug: default_request_household_slug)
      parent_membership = household.household_memberships.find_by!(account: users(:parent).person.account)
      jane = users(:jane).person
      jane.update!(household: household)
      jane_membership = household.household_memberships.create!(
        account: jane.account,
        person: jane,
        role: :member,
        status: :active
      )
      grant = household.person_access_grants.create!(
        household_membership: jane_membership,
        person: people(:child_user_person),
        access_level: :view,
        relationship_type: :parent,
        granted_by_membership: parent_membership,
        revoked_at: 1.day.ago
      )

      post person_carer_relationships_path(people(:child_user_person)),
           params: { carer_relationship: { email: users(:jane).email_address } }

      expect(grant.reload).to have_attributes(
        revoked_at: nil,
        access_level: 'manage',
        relationship_type: 'parent'
      )
    end

    it 'creates a parent invitation when the second parent does not have an account' do
      sign_in(users(:parent))

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: 'new.second.parent@example.com' } }
      end.to change(HouseholdInvitation, :count).by(1)

      invitation = HouseholdInvitation.find_by!(email: 'new.second.parent@example.com')
      grant = invitation.household_invitation_grants.sole
      expect(invitation.membership_role).to eq('member')
      expect(grant).to have_attributes(
        person: people(:child_user_person),
        access_level: 'manage',
        relationship_type: 'parent'
      )
      expect(response).to redirect_to(person_path(people(:child_user_person)))
    end

    it 'does not attach dependents to an existing non-parent invitation' do
      sign_in(users(:parent))
      household = Household.find_by!(slug: default_request_household_slug)
      invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: household.household_memberships.active.first,
        email: 'pending.carer@example.com'
      )
      invitation.household_invitation_grants.create!(
        household: household,
        person: people(:child_user_person),
        access_level: :record,
        relationship_type: :professional
      )

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: invitation.email } }
      end.not_to change(HouseholdInvitationGrant, :count)

      expect(invitation.reload.household_invitation_grants.sole.relationship_type).to eq('professional')
      expect(response).to have_http_status(:unprocessable_content)
    end

    it 'attaches the dependent to an existing pending parent invitation regardless of email case' do
      sign_in(users(:parent))
      household = Household.find_by!(slug: default_request_household_slug)
      invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: household.household_memberships.active.first,
        email: 'pending.second.parent@example.com'
      )
      write_legacy_email(invitation, :email, 'Pending.Second.Parent@example.com')

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: 'pending.second.parent@example.com' } }
      end.not_to change(HouseholdInvitation, :count)

      grant = invitation.reload.household_invitation_grants.sole
      expect(grant).to have_attributes(person: people(:child_user_person), relationship_type: 'parent')
      expect(response).to redirect_to(person_path(people(:child_user_person)))
    end

    it 'prevents parents from assigning another parent to an unrelated child' do
      sign_in(users(:parent))

      expect do
        post person_carer_relationships_path(people(:child_patient)),
             params: { carer_relationship: { email: users(:jane).email_address } }
      end.not_to change(CarerRelationship, :count)
    end

    it 'prevents parents from assigning a minor account with a parent role' do
      sign_in(users(:parent))

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: users(:child_user).email_address } }
      end.not_to change(CarerRelationship, :count)

      expect(response).to have_http_status(:unprocessable_content)
    end

    it 'prevents parents from assigning a dependent adult account with a parent role' do
      sign_in(users(:parent))
      user = create_dependent_adult_parent_user

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: user.email_address } }
      end.not_to change(CarerRelationship, :count)

      expect(response).to have_http_status(:unprocessable_content)
    end

    it 'prevents parents from assigning a self-managing adult account' do
      sign_in(users(:parent))

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: users(:adult_patient).email_address } }
      end.not_to change(CarerRelationship, :count)

      expect(response).to have_http_status(:unprocessable_content)
    end

    it 'prevents parents from assigning a clinician account' do
      sign_in(users(:parent))
      users(:doctor).person.update!(professional_title: 'doctor')

      expect do
        post person_carer_relationships_path(people(:child_user_person)),
             params: { carer_relationship: { email: users(:doctor).email_address } }
      end.not_to change(CarerRelationship, :count)

      expect(response).to have_http_status(:unprocessable_content)
    end
  end

  def create_dependent_adult_parent_user
    person = build_dependent_adult_parent_person
    person.save!
    User.create!(person: person, email_address: person.email)
  end

  def build_dependent_adult_parent_person
    Person.new(
      name: 'Dependent Adult Parent Role',
      email: 'dependent.adult.parent@example.com',
      date_of_birth: 30.years.ago.to_date,
      person_type: :dependent_adult,
      has_capacity: false,
      household: Household.find_by!(slug: default_request_household_slug)
    ).tap { |person| attach_dependent_adult_parent_links(person) }
  end

  def attach_dependent_adult_parent_links(person)
    person.location_memberships.build(location: household_location(locations(:home)))
    person.carer_relationships.build(
      carer: household_person(users(:parent).person),
      relationship_type: 'family_member',
      active: true
    )
  end

  def write_legacy_email(record, attribute, value)
    connection = record.class.connection
    table = connection.quote_table_name(record.class.table_name)
    column = connection.quote_column_name(attribute)
    sql = record.class.sanitize_sql_array(["UPDATE #{table} SET #{column} = ? WHERE id = ?", value, record.id])
    connection.execute(sql)
    record.reload
  end
end
