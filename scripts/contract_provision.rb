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
  delegated_account = Account.create!(email: "contract-delegated-#{nonce}@example.test", status: :verified,
                                      password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  delegated_person = household.people.create!(account: delegated_account, name: "Contract delegated #{nonce}",
                                              date_of_birth: 32.years.ago.to_date, person_type: :adult,
                                              has_capacity: true)
  User.create!(person: delegated_person, email_address: delegated_account.email, password: 'password', active: true)
  delegated_membership = household.household_memberships.create!(account: delegated_account, person: delegated_person,
                                                                 role: :member, status: :active)
  PersonAccessGrant.create!(household: household, household_membership: delegated_membership, person: managed_person,
                            access_level: :manage, relationship_type: :carer, granted_by_membership: membership)
  _delegated_session, delegated_access_token, = ApiSession.issue_for(
    account: delegated_account, household_membership: delegated_membership, device_name: 'contract-delegated'
  )
  view_owner_account = Account.create!(email: "contract-view-owner-#{nonce}@example.test", status: :verified,
                                       password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  view_owner_person = household.people.create!(account: view_owner_account, name: "Contract view owner #{nonce}",
                                               date_of_birth: 31.years.ago.to_date, person_type: :adult,
                                               has_capacity: true)
  User.create!(person: view_owner_person, email_address: view_owner_account.email, password: 'password', active: true)
  view_owner_membership = household.household_memberships.create!(account: view_owner_account,
                                                                  person: view_owner_person, role: :owner,
                                                                  status: :active)
  PersonAccessGrant.create!(household: household, household_membership: view_owner_membership, person: managed_person,
                            access_level: :view, relationship_type: :carer, granted_by_membership: membership)
  _view_owner_session, view_owner_access_token, = ApiSession.issue_for(
    account: view_owner_account, household_membership: view_owner_membership, device_name: 'contract-view-owner'
  )
  grant_target_account = Account.create!(email: "contract-grant-target-#{nonce}@example.test", status: :verified)
  grant_target_membership = household.household_memberships.create!(account: grant_target_account, role: :member,
                                                                    status: :active)
  primary_location = Location.create!(household: household, name: "Contract shelf #{nonce}")
  foreign_location = Location.create!(household: foreign_household, name: "Contract foreign shelf #{nonce}")
  managed_medication = Medication.create!(household: household, location: primary_location,
                                          name: "Contract managed medicine #{nonce}", dose_amount: '2',
                                          dose_unit: 'ml', current_supply: '50', reorder_threshold: '5')
  hidden_medication = Medication.create!(household: household, location: primary_location,
                                         name: "Contract hidden medicine #{nonce}", dose_amount: '2',
                                         dose_unit: 'ml', current_supply: '50', reorder_threshold: '5')
  foreign_medication = Medication.create!(household: foreign_household, location: foreign_location,
                                          name: "Contract foreign medicine #{nonce}", dose_amount: '2',
                                          dose_unit: 'ml', current_supply: '50', reorder_threshold: '5')
  foreign_dosage = foreign_medication.dosage_records.create!(amount: '1', unit: 'ml', frequency: 'daily',
                                                             default_max_daily_doses: 4,
                                                             default_min_hours_between_doses: '4',
                                                             default_dose_cycle: :daily)
  managed_assignment = PersonMedication.create!(household: household, person: managed_person,
                                                medication: managed_medication, administration_kind: :as_needed)
  hidden_assignment = PersonMedication.create!(household: household, person: hidden_person,
                                               medication: hidden_medication, administration_kind: :as_needed)
  foreign_assignment = PersonMedication.create!(household: foreign_household, person: foreign_account.person,
                                                medication: foreign_medication, administration_kind: :as_needed)
  hidden_assignment.pause!
  foreign_assignment.pause!
  hidden_pause_period = hidden_assignment.medication_pause_periods.sole
  foreign_pause_period = foreign_assignment.medication_pause_periods.sole
  hidden_dosage = hidden_medication.dosage_records.create!(amount: '1', unit: 'ml', frequency: 'daily',
                                                           default_max_daily_doses: 4,
                                                           default_min_hours_between_doses: '4',
                                                           default_dose_cycle: :daily)
  managed_schedule = Schedule.create!(household: household, person: managed_person, medication: managed_medication,
                                      dose_amount: '1', dose_unit: 'ml', frequency: 'Daily',
                                      start_date: '2026-02-25', end_date: '2099-12-31')
  historical_location = Location.create!(household: household, name: "Contract historical shelf #{nonce}")
  historical_medication = Medication.create!(household: household, location: historical_location,
                                             name: "Contract historical medicine #{nonce}", dose_amount: '1',
                                             dose_unit: 'ml', current_supply: '5')
  historical_schedule = Schedule.create!(household: household, person: managed_person,
                                         medication: historical_medication, dose_amount: '1', dose_unit: 'ml',
                                         frequency: 'Daily', start_date: '2026-02-25', end_date: '2099-12-31')
  historical_schedule.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1,
                                                          outcome: 'open')
  hidden_schedule = Schedule.create!(household: household, person: hidden_person, medication: hidden_medication,
                                     dose_amount: '1', dose_unit: 'ml', frequency: 'Daily',
                                     start_date: '2026-02-25', end_date: '2099-12-31')
  foreign_schedule = Schedule.create!(household: foreign_household, person: foreign_account.person,
                                      medication: foreign_medication, dose_amount: '1', dose_unit: 'ml',
                                      frequency: 'Daily', start_date: '2026-02-25', end_date: '2099-12-31')
  hidden_health_event = HealthEvent.create!(household: household, person: hidden_person, event_kind: :illness,
                                            title: "Contract hidden event #{nonce}", started_on: '2026-02-25')
  foreign_health_event = HealthEvent.create!(household: foreign_household, person: foreign_account.person,
                                             event_kind: :illness, title: "Contract foreign event #{nonce}",
                                             started_on: '2026-02-25')
  review_partner = Medication.create!(household: household, location: primary_location,
                                      name: "Contract review partner #{nonce}", dose_amount: '1', dose_unit: 'tablet')
  foreign_review_partner = Medication.create!(household: foreign_household, location: foreign_location,
                                              name: "Contract foreign review partner #{nonce}", dose_amount: '1',
                                              dose_unit: 'tablet')
  review_evidence = { 'high' => 'high', 'moderate' => 'moderate', 'low' => 'low',
                      'edit' => 'unknown', 'invalid' => 'unknown' }.to_h do |label, risk|
    evidence = MedicationReviewEvidenceRecord.create!(source_name: 'Contract source',
                                                      source_record_id: "contract-#{label}-#{nonce}",
                                                      source_url: 'https://example.test/contract-evidence',
                                                      retrieved_on: '2026-02-25', product_name: 'Contract medicine',
                                                      label_section: 'warnings',
                                                      evidence_text: "Contract evidence #{risk}",
                                                      risk_level: risk, match_confidence: risk,
                                                      match_status: 'not_pairwise')
    [label, evidence]
  end
  review_attributes = lambda do |owner, subject, primary, partner, evidence, status|
    MedicationReviewPrompt.create!(household: owner, person: subject, primary_medication: primary,
                                   interacting_medication: partner, evidence_record: evidence,
                                   risk_level: evidence.risk_level, match_confidence: evidence.match_confidence,
                                   primary_medication_name: primary.name, interacting_medication_name: partner.name,
                                   evidence_source_name: evidence.source_name,
                                   evidence_source_url: evidence.source_url,
                                   evidence_source_checked_on: evidence.retrieved_on,
                                   evidence_source_version: 'contract-v1',
                                   evidence_source_effective_on: evidence.retrieved_on,
                                   matched_term: 'Contract medicine', match_type: 'reviewed_pair',
                                   source_instruction: 'Discuss with a practitioner',
                                   match_reason: 'Contract pair', evidence_text: evidence.evidence_text,
                                   status: status)
  end
  managed_review_prompt = review_attributes.call(household, managed_person, managed_medication, review_partner,
                                                 review_evidence.fetch('high'), 'needs_review')
  second_review_prompt = review_attributes.call(household, managed_person, managed_medication, review_partner,
                                                review_evidence.fetch('moderate'), 'needs_review')
  low_signal_review_prompt = review_attributes.call(household, managed_person, managed_medication, review_partner,
                                                    review_evidence.fetch('low'), 'hidden_low_signal')
  edit_review_prompt = review_attributes.call(household, managed_person, managed_medication, review_partner,
                                              review_evidence.fetch('edit'), 'needs_review')
  invalid_review_prompt = review_attributes.call(household, managed_person, managed_medication, review_partner,
                                                 review_evidence.fetch('invalid'), 'needs_review')
  hidden_review_prompt = review_attributes.call(household, hidden_person, hidden_medication, review_partner,
                                                review_evidence.fetch('high'), 'needs_review')
  foreign_review_prompt = review_attributes.call(foreign_household, foreign_account.person, foreign_medication,
                                                 foreign_review_partner, review_evidence.fetch('high'), 'needs_review')
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
    delegated_access_token: delegated_access_token,
    view_owner_access_token: view_owner_access_token,
    care_access_token: care_access_token,
    grant_target_membership_id: grant_target_membership.id,
    primary_location_id: primary_location.id,
    primary_location_portable_id: primary_location.portable_id,
    historical_location_portable_id: historical_location.portable_id,
    foreign_location_id: foreign_location.id,
    foreign_location_name: foreign_location.name,
    managed_medication_id: managed_medication.id,
    managed_medication_portable_id: managed_medication.portable_id,
    hidden_medication_id: hidden_medication.id,
    foreign_medication_id: foreign_medication.id,
    foreign_medication_portable_id: foreign_medication.portable_id,
    foreign_medication_name: foreign_medication.name,
    managed_assignment_id: managed_assignment.id,
    hidden_assignment_id: hidden_assignment.id,
    hidden_assignment_portable_id: hidden_assignment.portable_id,
    foreign_assignment_id: foreign_assignment.id,
    foreign_assignment_portable_id: foreign_assignment.portable_id,
    hidden_pause_period_id: hidden_pause_period.portable_id,
    foreign_pause_period_id: foreign_pause_period.portable_id,
    managed_schedule_id: managed_schedule.id,
    hidden_schedule_id: hidden_schedule.id,
    foreign_schedule_id: foreign_schedule.id,
    foreign_dosage_id: foreign_dosage.id,
    hidden_dosage_id: hidden_dosage.id,
    hidden_health_event_id: hidden_health_event.id,
    foreign_health_event_id: foreign_health_event.id,
    managed_review_prompt_id: managed_review_prompt.id,
    second_review_prompt_id: second_review_prompt.id,
    low_signal_review_prompt_id: low_signal_review_prompt.id,
    edit_review_prompt_id: edit_review_prompt.id,
    invalid_review_prompt_id: invalid_review_prompt.id,
    hidden_review_prompt_id: hidden_review_prompt.id,
    foreign_review_prompt_id: foreign_review_prompt.id
  }
end
File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
  file.write(JSON.generate(fixture))
end
puts "Wrote disposable contract fixture to #{path}"
