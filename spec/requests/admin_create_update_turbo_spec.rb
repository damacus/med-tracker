# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin create and update turbo flows' do
  fixtures :accounts, :people, :users, :locations, :carer_relationships

  let(:admin) { users(:admin) }
  let(:household) { Household.find_by!(slug: default_url_options.fetch(:household_slug)) }

  before do
    sign_in(admin)
    attach_fixture_users_to_household
  end

  describe 'POST /admin/invitations' do
    it 'renders role-aware dependent assignment controls on the invitation form' do
      get admin_invitations_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-controller="dependent-assignment"')
      expect(response.body).to include('data-dependent-assignment-target="field"')
      expect(response.body).to include('name="invitation[dependent_ids][]"')
    end

    it 'returns turbo_stream and updates invitations container and flash on success' do
      post admin_invitations_path,
           params: {
             invitation: {
               email: 'new.user@example.com',
               membership_role: 'member',
               access_level: 'record',
               relationship_type: 'professional'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="admin_invitations"')
      expect(response.body).to include('target="flash"')
    end

    it 'stores selected dependents for parent invitations' do
      dependent = people(:child_patient)

      post admin_invitations_path,
           params: {
             invitation: {
               email: 'second.parent@example.com',
               membership_role: 'member',
               access_level: 'manage',
               relationship_type: 'parent',
               dependent_ids: [dependent.id]
             }
           }

      invitation = HouseholdInvitation.find_by!(email: 'second.parent@example.com')
      grant = invitation.household_invitation_grants.sole
      expect(grant).to have_attributes(person: dependent, access_level: 'manage', relationship_type: 'parent')
    end

    it 'stores selected dependents for carer invitations' do
      dependent = people(:child_user_person)

      post admin_invitations_path,
           params: {
             invitation: {
               email: 'invited.carer@example.com',
               membership_role: 'member',
               access_level: 'record',
               relationship_type: 'professional',
               dependent_ids: [dependent.id]
             }
           }

      invitation = HouseholdInvitation.find_by!(email: 'invited.carer@example.com')
      grant = invitation.household_invitation_grants.sole
      expect(grant).to have_attributes(person: dependent, access_level: 'record', relationship_type: 'professional')
    end

    it 'returns unprocessable content when inviting an owner' do
      post admin_invitations_path,
           params: { invitation: { email: 'owner.user@example.com', membership_role: 'owner' } },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Membership role')
    end
  end

  describe 'POST /admin/invitations/:id/resend' do
    let!(:invitation) do
      create(
        :household_invitation,
        household: household,
        invited_by_membership: household.household_memberships.owner.sole,
        email: 'existing.user@example.com',
        expires_at: 1.day.ago
      )
    end

    it 'returns turbo_stream, rotates the token, records an audit event, and updates flash on success' do
      original_digest = invitation.token_digest

      post resend_admin_invitation_path(invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="admin_invitations"')
      expect(response.body).to include('target="flash"')
      expect(invitation.reload.token_digest).not_to eq(original_digest)
      expect(invitation.versions.last.event).to eq('resend')
      expect(invitation.versions.last.whodunnit).to eq(admin.id.to_s)
    end

    it 'returns an alert and leaves accepted invitations unchanged' do
      accepted_invitation = create(
        :household_invitation,
        :accepted,
        household: household,
        invited_by_membership: household.household_memberships.owner.sole,
        email: 'accepted.user@example.com'
      )

      post resend_admin_invitation_path(accepted_invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('Accepted invitations cannot be resent')
      expect(response.body).not_to include('Invitation resent')
      expect(accepted_invitation.reload.versions.last.event).not_to eq('resend')
    end

    it 'returns an alert and leaves the invitation unchanged when it has been revoked' do
      revoked_invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: household.household_memberships.owner.sole,
        email: 'revoked.user@example.com',
        revoked_at: Time.current
      )
      original_digest = revoked_invitation.token_digest
      original_expires_at = revoked_invitation.expires_at

      post resend_admin_invitation_path(revoked_invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include(
        'This invitation could not be resent. A pending invitation for this email may already exist.'
      )
      revoked_invitation.reload
      expect(revoked_invitation.token_digest).to eq(original_digest)
      expect(revoked_invitation.expires_at.to_i).to eq(original_expires_at.to_i)
    end
  end

  describe 'POST /admin/users' do
    it 'renders role-aware dependent assignment controls on the user form' do
      foreign_household = Household.create!(name: 'Foreign User Form Household', slug: 'foreign-user-form-household')
      foreign_dependent = Person.new(
        name: 'Foreign Dependent Picker Leak',
        date_of_birth: 12.years.ago.to_date,
        person_type: :minor,
        has_capacity: false,
        household: foreign_household
      )
      foreign_dependent.save!(validate: false)

      get new_admin_user_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-controller="dependent-assignment"')
      expect(response.body).to include('data-dependent-assignment-target="field"')
      expect(response.body).to include('name="user[dependent_ids][]"')
      expect(response.body).not_to include("user_dependent_#{foreign_dependent.id}")
      expect(response.body).not_to include('Foreign Dependent Picker Leak')
    end

    it 'returns turbo_stream and replaces users index and flash on success' do
      post admin_users_path,
           params: {
             user: {
               email_address: 'turbo.new.user@example.com',
               password: 'password',
               password_confirmation: 'password',
               membership_role: 'member',
               dependent_access_level: 'record',
               dependent_relationship_type: 'professional',
               person_attributes: {
                 name: 'Turbo New User',
                 date_of_birth: '1990-01-01',
                 location_ids: [locations(:home).id]
               }
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="main-content"')
      expect(response.body).to include('id="admin-users-frame"')
      expect(response.body).to include('target="flash"')
      expect(response.body).to include('Turbo New User')
    end

    it 'links a new parent to selected existing children' do
      dependent = people(:child_patient)

      expect do
        post admin_users_path,
             params: {
               user: {
                 email_address: 'linked.parent@example.com',
                 password: 'password',
                 password_confirmation: 'password',
                 membership_role: 'member',
                 dependent_access_level: 'manage',
                 dependent_relationship_type: 'parent',
                 dependent_ids: [dependent.id],
                 person_attributes: {
                   name: 'Linked Parent',
                   date_of_birth: '1990-01-01',
                   location_ids: [locations(:home).id]
                 }
               }
             }
      end.to change(CarerRelationship, :count).by(1)

      parent = User.find_by!(email_address: 'linked.parent@example.com').person
      relationship = CarerRelationship.find_by!(carer: parent, patient: dependent)
      expect(relationship.relationship_type).to eq('parent')
      expect(relationship.active).to be true
      grant = PersonAccessGrant.find_by!(person: dependent,
                                         household_membership: parent.account.household_memberships.sole)
      expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'parent')
    end

    it 'links a new carer to selected existing children' do
      dependent = people(:child_user_person)

      expect do
        post admin_users_path,
             params: {
               user: {
                 email_address: 'linked.carer@example.com',
                 password: 'password',
                 password_confirmation: 'password',
                 membership_role: 'member',
                 dependent_access_level: 'record',
                 dependent_relationship_type: 'professional',
                 dependent_ids: [dependent.id],
                 person_attributes: {
                   name: 'Linked Carer',
                   date_of_birth: '1990-01-01',
                   location_ids: [locations(:home).id]
                 }
               }
             }
      end.to change(CarerRelationship, :count).by(1)

      carer = User.find_by!(email_address: 'linked.carer@example.com').person
      relationship = CarerRelationship.find_by!(carer: carer, patient: dependent)
      expect(relationship.relationship_type).to eq('professional_carer')
      expect(relationship.active).to be true
      grant = PersonAccessGrant.find_by!(person: dependent,
                                         household_membership: carer.account.household_memberships.sole)
      expect(grant).to have_attributes(access_level: 'record', relationship_type: 'professional')
    end

    it 'returns unprocessable content on validation failure' do
      post admin_users_path,
           params: {
             user: {
               email_address: users(:jane).email_address,
               password: 'password',
               password_confirmation: 'password',
               membership_role: 'member',
               dependent_access_level: 'record',
               dependent_relationship_type: 'professional',
               person_attributes: {
                 name: 'Duplicate Email User',
                 date_of_birth: '1990-01-01',
                 location_ids: [locations(:home).id]
               }
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('role="alert"')
      expect(response.body).to include('id="user_email_address_error"')
      expect(response.body).to include('aria-describedby="user_email_address_error"')
      expect(response.body).to include('aria-invalid')
    end
  end

  describe 'PATCH /admin/users/:id' do
    it 'renders role changes through the dedicated membership role form on edit' do
      get edit_admin_user_path(users(:jane))

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(%(action="#{membership_role_admin_user_path(users(:jane))}"))
      expect(response.body).to include('name="membership[role]"')
      expect(response.body).not_to include('name="user[membership_role]"')
    end

    it 'returns turbo_stream and replaces users index and flash on success' do
      patch admin_user_path(users(:jane)),
            params: {
              user: {
                email_address: users(:jane).email_address,
                membership_role: 'member',
                dependent_access_level: 'manage',
                dependent_relationship_type: 'parent',
                person_attributes: {
                  id: users(:jane).person.id,
                  name: 'Jane Turbo Update',
                  date_of_birth: users(:jane).person.date_of_birth.to_s,
                  location_ids: [locations(:home).id]
                }
              }
            },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="main-content"')
      expect(response.body).to include('id="admin-users-frame"')
      expect(response.body).to include('target="flash"')
      expect(response.body).to include('Jane Turbo Update')
      expect(users(:jane).person.reload.name).to eq('Jane Turbo Update')
    end

    it 'returns unprocessable content on validation failure' do
      patch admin_user_path(users(:jane)),
            params: {
              user: {
                email_address: '',
                person_attributes: {
                  id: users(:jane).person.id,
                  name: users(:jane).person.name,
                  date_of_birth: users(:jane).person.date_of_birth.to_s
                }
              }
            },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('id="user_email_address_error"')
      expect(response.body).to include('aria-invalid')
    end

    it 'does not add selected existing children through the generic user update' do
      parent = users(:parent)
      dependent = people(:child_patient)

      expect do
        patch admin_user_path(parent),
              params: {
                user: {
                  email_address: parent.email_address,
                  membership_role: 'member',
                  dependent_access_level: 'manage',
                  dependent_relationship_type: 'parent',
                  dependent_ids: [dependent.id],
                  person_attributes: {
                    id: parent.person.id,
                    name: parent.person.name,
                    date_of_birth: parent.person.date_of_birth.to_s,
                    location_ids: [locations(:home).id]
                  }
                }
              }
      end.not_to change(CarerRelationship, :count)

      expect(CarerRelationship.exists?(carer: parent.person, patient: dependent)).to be(false)
    end

    it 'does not reactivate revoked dependent access grants through the generic user update' do
      parent = users(:parent)
      dependent = people(:child_patient)
      membership = household.household_memberships.find_by!(account: parent.person.account)
      grant = household.person_access_grants.create!(
        household_membership: membership,
        person: dependent,
        access_level: :view,
        relationship_type: :parent,
        granted_by_membership: household.household_memberships.owner.sole,
        revoked_at: 1.day.ago
      )

      patch admin_user_path(parent),
            params: {
              user: {
                email_address: parent.email_address,
                membership_role: 'member',
                dependent_access_level: 'manage',
                dependent_relationship_type: 'parent',
                dependent_ids: [dependent.id],
                person_attributes: {
                  id: parent.person.id,
                  name: parent.person.name,
                  date_of_birth: parent.person.date_of_birth.to_s,
                  location_ids: [locations(:home).id]
                }
              }
            }

      expect(grant.reload).to have_attributes(revoked_at: be_present, access_level: 'view', relationship_type: 'parent')
    end
  end

  describe 'POST /admin/carer_relationships' do
    it 'returns turbo_stream and updates modal, rows, and flash on success' do
      post admin_carer_relationships_path,
           params: {
             carer_relationship: {
               carer_id: people(:jane).id,
               patient_id: people(:john).id,
               relationship_type: 'family_member'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('target="carer_relationships_rows"')
      expect(response.body).to include('target="flash"')
    end

    it 'keeps membership role separate from relationship access grants' do
      doctor = users(:doctor)
      patient = create(:person, household: household, name: 'Professional Grant Patient')
      existing_membership = household.household_memberships.find_by(account: doctor.person.account)
      existing_membership&.person_access_grants&.delete_all
      existing_membership&.destroy!

      post admin_carer_relationships_path,
           params: {
             carer_relationship: {
               carer_id: doctor.person.id,
               patient_id: patient.id,
               relationship_type: 'professional_carer'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      membership = household.household_memberships.find_by!(account: doctor.person.account)
      grant = household.person_access_grants.find_by!(household_membership: membership, person: patient)
      expect(membership.role).to eq('member')
      expect(grant).to have_attributes(access_level: 'record', relationship_type: 'professional')
      event = SecurityAuditEvent.where(event_type: 'household_access.membership_created').order(:id).last
      expect(event.metadata).to include(
        'target_membership_id' => membership.id,
        'new_state' => include('permissions_version' => 1)
      )
    end
  end

  describe 'GET /admin/carer_relationships/new' do
    it 'renders the standalone HTML page with the application layout' do
      get new_admin_carer_relationship_path

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/html')
      expect(response.body).to include('<html')
      expect(response.body).to include('stylesheet')
      expect(response.body).to include('New Carer Relationship')
    end

    it 'returns turbo_stream and replaces modal frame' do
      get new_admin_carer_relationship_path,
          headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('carer_relationship_carer_id')
    end
  end

  def attach_fixture_users_to_household
    attach_admin_to_household

    User.where.not(id: admin.id).includes(person: :account).find_each do |user|
      attach_member_to_household(user)
    end
  end

  def attach_admin_to_household
    attach_person_to_household(admin.person)
    upsert_household_membership(admin.person, :owner)
  end

  def attach_member_to_household(user)
    return unless user.person&.account

    attach_person_to_household(user.person)
    upsert_household_membership(user.person, :member)
  end

  def attach_person_to_household(person)
    person.household = household
    person.save!(validate: false)
  end

  def upsert_household_membership(person, role)
    household.household_memberships.find_or_initialize_by(account: person.account).tap do |membership|
      membership.person = person
      membership.role = role
      membership.status = :active
      membership.save!
    end
  end
end
