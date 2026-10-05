# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person avatars' do
  before do
    grant_owner_avatar_access
    avatar_person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')
  end

  it 'serves an uploaded avatar to an authorized household member' do
    sign_in(owner_user)

    get person_avatar_path(person_id: avatar_person.id)

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('image/png')
    expect(response.body).to eq('avatar')
  end

  it 'does not serve an uploaded avatar to a member without access to the person' do
    create_avatar_member_membership
    sign_in(member_user)

    get person_avatar_path(person_id: avatar_person.id)

    expect(response).to have_http_status(:not_found).or have_http_status(:forbidden).or have_http_status(:found)
  end

  it 'does not serve an uploaded avatar through a foreign household route' do
    sign_in(owner_user)
    foreign_household = Household.create!(name: 'Foreign Avatar Household', slug: 'foreign-avatar-household')

    get "/households/#{foreign_household.slug}/people/#{avatar_person.id}/avatar"

    expect(response).to have_http_status(:found)
  end

  def owner_account
    @owner_account ||= Account.create!(
      email: 'avatar-owner@example.test',
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :verified
    )
  end

  def household
    @household ||= Household.create_with_owner!(
      name: 'Avatar Household',
      owner_account: owner_account,
      owner_person_attributes: avatar_person_attributes('Avatar Owner')
    )
  end

  def owner_membership
    @owner_membership ||= household.household_memberships.find_by!(account: owner_account)
  end

  def owner_user
    @owner_user ||= User.create!(
      person: owner_membership.person,
      email_address: owner_account.email,
      password: 'password'
    )
  end

  def avatar_person
    @avatar_person ||= household.people.create!(avatar_person_attributes('Avatar Patient'))
  end

  def member_account
    @member_account ||= Account.create!(
      email: 'avatar-member@example.test',
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :verified
    )
  end

  def member_person
    @member_person ||= household.people.create!(
      avatar_person_attributes('Avatar Member').merge(account: member_account)
    )
  end

  def member_user
    @member_user ||= User.create!(person: member_person, email_address: member_account.email, password: 'password')
  end

  def grant_owner_avatar_access
    household.person_access_grants.create!(
      household_membership: owner_membership,
      person: avatar_person,
      access_level: :manage,
      relationship_type: :family_member,
      granted_by_membership: owner_membership
    )
  end

  def create_avatar_member_membership
    household.household_memberships.create!(
      account: member_account,
      person: member_person,
      role: :member,
      status: :active
    )
  end

  def avatar_person_attributes(name)
    {
      name: name,
      date_of_birth: 25.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    }
  end
end
