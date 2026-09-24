require 'json'
require 'securerandom'

abort 'Contract fixtures require the Rails test environment' unless Rails.env.test?

path = Pathname.new(ENV.fetch('CONTRACT_FIXTURE_PATH'))
fixture_root = Rails.root.join('tmp/contract-tests').realpath
abort 'Contract fixture path must be in its own run directory' unless
  path.absolute? && path.basename.to_s == 'fixture.json' && path.dirname.dirname.realpath == fixture_root &&
  path.dirname.basename.to_s.start_with?('run.')
nonce = SecureRandom.hex(12)

def create_household(nonce, label)
  email = "contract-#{label}-#{nonce}@example.test"
  account = Account.create!(email: email, status: :verified,
                            password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  household = Household.create_with_owner!(
    name: "Contract #{label} #{nonce}",
    owner_account: account,
    owner_person_attributes: {
      name: "Contract #{label}", date_of_birth: 30.years.ago.to_date,
      person_type: :adult, has_capacity: true
    }
  )
  user = User.create!(person: account.person, email_address: email, password: 'password', active: true)
  [account, household, user]
end

fixture = ActiveRecord::Base.transaction do
  account, household, user = create_household(nonce, 'primary')
  foreign_account, foreign_household, = create_household(nonce, 'foreign')
  membership = account.household_memberships.find_by!(household: household)
  managed_person = household.people.create!(name: "Contract managed #{nonce}", date_of_birth: 35.years.ago.to_date,
                                            person_type: :adult, has_capacity: true)
  hidden_person = household.people.create!(name: "Contract hidden #{nonce}", date_of_birth: 36.years.ago.to_date,
                                           person_type: :adult, has_capacity: true)
  PersonAccessGrant.create!(household: household, household_membership: membership, person: managed_person,
                            access_level: :manage, relationship_type: :family_member,
                            granted_by_membership: membership)
  care_account = Account.create!(email: "contract-care-#{nonce}@example.test", status: :verified,
                                 password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  care_person = household.people.create!(account: care_account, name: "Contract carer #{nonce}",
                                         date_of_birth: 33.years.ago.to_date, person_type: :adult, has_capacity: true)
  User.create!(person: care_person, email_address: care_account.email, password: 'password', active: true)
  care_membership = household.household_memberships.create!(account: care_account, person: care_person,
                                                            role: :owner, status: :active)
  PersonAccessGrant.create!(household: household, household_membership: care_membership, person: managed_person,
                            access_level: :manage, relationship_type: :family_member,
                            granted_by_membership: membership)
  view_account = Account.create!(email: "contract-view-#{nonce}@example.test", status: :verified,
                                 password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  view_person = household.people.create!(account: view_account, name: "Contract viewer #{nonce}",
                                         date_of_birth: 32.years.ago.to_date, person_type: :adult, has_capacity: true)
  User.create!(person: view_person, email_address: view_account.email, password: 'password', active: true)
  view_membership = household.household_memberships.create!(account: view_account, person: view_person,
                                                            role: :member, status: :active)
  PersonAccessGrant.create!(household: household, household_membership: view_membership, person: managed_person,
                            access_level: :view, relationship_type: :carer,
                            granted_by_membership: membership)
  _view_session, view_access_token, = ApiSession.issue_for(
    account: view_account, household_membership: view_membership, device_name: 'contract-view'
  )
  grant_target_account = Account.create!(email: "contract-grant-target-#{nonce}@example.test", status: :verified)
  grant_target_membership = household.household_memberships.create!(account: grant_target_account, role: :member,
                                                                    status: :active)
  primary_location = Location.create!(household: household, name: "Contract shelf #{nonce}")
  foreign_location = Location.create!(household: foreign_household, name: "Contract foreign shelf #{nonce}")
  session, access_token, = ApiSession.issue_for(
    account: account, household_membership: membership, device_name: 'contract-tests'
  )
  revocable_session, revocable_access_token, = ApiSession.issue_for(
    account: account, household_membership: membership, device_name: 'contract-revocable'
  )
  _logout_session, logout_access_token, = ApiSession.issue_for(
    account: account, household_membership: membership, device_name: 'contract-logout'
  )
  expired_session, expired_access_token, = ApiSession.issue_for(
    account: account, household_membership: membership, device_name: 'contract-expired'
  )
  expired_session.update!(access_expires_at: 1.minute.ago)

  locked_account, _locked_household, = create_household(nonce, 'locked')
  locked_membership = locked_account.household_memberships.active.sole
  _locked_session, locked_access_token, = ApiSession.issue_for(
    account: locked_account, household_membership: locked_membership, device_name: 'contract-locked'
  )
  AccountLockout.create!(account: locked_account, key: SecureRandom.hex(16), deadline: 30.minutes.from_now)

  oauth_client_id = "contract-mobile-#{nonce}"
  oauth_redirect_uri = 'io.damacus.medtracker.contract:/oauth2redirect'
  oauth_application = OauthApplication.create!(name: 'Contract mobile', client_id: oauth_client_id, client_kind: :mobile,
                                               redirect_uri: oauth_redirect_uri, scopes: 'medtracker offline_access',
                                               token_endpoint_auth_method: 'none')
  care_access_token = "contract-care-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: care_account, oauth_application: oauth_application, client_kind: :mobile,
                     scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(care_access_token))

  {
    access_token: access_token,
    account_id: account.id,
    primary_email: account.email,
    user_id: user.id,
    household_id: household.id,
    household_name: household.name,
    foreign_household_id: foreign_household.id,
    foreign_email: "contract-foreign-#{nonce}@example.test",
    session_id: session.id,
    revocable_session_id: revocable_session.id,
    revocable_access_token: revocable_access_token,
    logout_access_token: logout_access_token,
    expired_access_token: expired_access_token,
    locked_access_token: locked_access_token,
    oauth_client_id: oauth_client_id,
    oauth_redirect_uri: oauth_redirect_uri,
    user_person_id: account.person.id,
    managed_person_id: managed_person.id,
    managed_person_portable_id: managed_person.portable_id,
    hidden_person_id: hidden_person.id,
    foreign_person_id: foreign_account.person.id,
    foreign_person_portable_id: foreign_account.person.portable_id,
    foreign_person_name: foreign_account.person.name,
    view_access_token: view_access_token,
    care_access_token: care_access_token,
    grant_target_membership_id: grant_target_membership.id,
    primary_location_id: primary_location.id,
    primary_location_portable_id: primary_location.portable_id,
    foreign_location_id: foreign_location.id,
    foreign_location_name: foreign_location.name
  }
end
File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
  file.write(JSON.generate(fixture))
end
puts "Wrote disposable contract fixture to #{path}"
