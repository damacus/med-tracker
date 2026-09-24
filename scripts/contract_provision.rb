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
  account = Account.create!(email: email, status: :verified)
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
  _foreign_account, foreign_household, = create_household(nonce, 'foreign')
  membership = account.household_memberships.find_by!(household: household)
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
  OauthApplication.create!(name: 'Contract mobile', client_id: oauth_client_id, client_kind: :mobile,
                           redirect_uri: oauth_redirect_uri, scopes: 'medtracker offline_access',
                           token_endpoint_auth_method: 'none')

  {
    access_token: access_token,
    account_id: account.id,
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
    oauth_redirect_uri: oauth_redirect_uri
  }
end
File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
  file.write(JSON.generate(fixture))
end
puts "Wrote disposable contract fixture to #{path}"
