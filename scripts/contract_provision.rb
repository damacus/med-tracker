require 'json'
require 'securerandom'

abort 'Contract fixtures require the Rails test environment' unless Rails.env.test?

path = Rails.root.join('tmp/contract-tests/fixture.json')
FileUtils.mkdir_p(path.dirname)
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
  _session, access_token, = ApiSession.issue_for(
    account: account, household_membership: membership, device_name: 'contract-tests'
  )

  {
    access_token: access_token,
    user_id: user.id,
    household_id: household.id,
    foreign_household_id: foreign_household.id,
    foreign_email: "contract-foreign-#{nonce}@example.test"
  }
end
File.unlink(path) if path.exist? || path.symlink?
File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
  file.write(JSON.generate(fixture))
end
puts "Wrote disposable contract fixture to #{path}"
