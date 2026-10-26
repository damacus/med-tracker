raise 'Benchmark database is not empty' if Account.exists? || Household.exists?
ActiveRecord::Base.transaction do
  account = Account.create!(email: 'persistence@example.test', status: :verified,
    password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  household = Household.create!(name: 'Synthetic household', slug: 'persistence-fixture',
    timezone: 'Europe/London', created_by_account: account)
  person = Person.create!(name: 'Synthetic adult', date_of_birth: '1985-02-03',
    person_type: :adult, has_capacity: true, household: household, account: account)
  User.create!(person: person, email_address: account.email, password: 'password')
  membership = HouseholdMembership.create!(household: household, account: account,
    person: person, role: :administrator, status: :active)
  PersonAccessGrant.create!(household: household, household_membership: membership,
    person: person, access_level: :manage, relationship_type: :self,
    granted_by_membership: membership)
  location = household.locations.first!
  medication = Medication.create!(household: household, location: location,
    name: 'Synthetic tablets', current_supply: 10, dose_amount: 2, dose_unit: 'tablet',
    default_schedule_type: :prn, reorder_threshold: 10)
  PersonMedication.create!(household: household, person: person, medication: medication,
    dose_amount: 2, dose_unit: 'tablet', active: true, administration_kind: :as_needed)
end
puts({accounts: Account.count, households: Household.count, people: Person.count,
  medications: Medication.count, treatments: PersonMedication.count}.to_json)
