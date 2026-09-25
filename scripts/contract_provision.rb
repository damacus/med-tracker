require 'json'
require 'securerandom'
require 'stringio'

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

def create_admin_member(household, nonce, label, role: :member)
  email = "contract-#{label}-#{nonce}@example.test"
  account = Account.create!(email: email, status: :verified,
                            password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  person = household.people.create!(account: account, name: "Contract #{label}",
                                    date_of_birth: 30.years.ago.to_date,
                                    person_type: :adult, has_capacity: true)
  User.create!(person: person, email_address: email, password: 'password', active: true)
  membership = household.household_memberships.create!(account: account, person: person,
                                                       role: role, status: :active)
  _session, token, = ApiSession.issue_for(account: account, household_membership: membership,
                                         device_name: "contract-#{label}")
  [membership, token]
end

fixture = ActiveRecord::Base.transaction do
  account, household, user = create_household(nonce, 'primary')
  platform_account, platform_household, = create_household(nonce, 'platform')
  PlatformAdmin.create!(account: platform_account)
  platform_target_membership, = create_admin_member(platform_household, nonce, 'platform-target')
  platform_promote_membership, = create_admin_member(platform_household, nonce, 'platform-promote')
  platform_denied_membership, = create_admin_member(platform_household, nonce, 'platform-denied')
  platform_support_account, platform_support_household, = create_household(nonce, 'platform-support')
  platform_support_membership = platform_support_account.household_memberships.find_by!(household: platform_support_household)
  _platform_support_api_session, platform_support_audit_token, = ApiSession.issue_for(
    account: platform_support_account, household_membership: platform_support_membership,
    device_name: 'contract-platform-audit'
  )
  foreign_account, foreign_household, = create_household(nonce, 'foreign')
  web_device_account, web_device_household, = create_household(nonce, 'web-device')
  web_device_membership = web_device_account.household_memberships.find_by!(household: web_device_household)
  _web_device_session, web_device_access_token, = ApiSession.issue_for(
    account: web_device_account, household_membership: web_device_membership, device_name: 'contract-web-device'
  )
  push_api_account, push_api_household, = create_household(nonce, 'push-api')
  push_api_membership = push_api_account.household_memberships.find_by!(household: push_api_household)
  _push_api_session, push_api_access_token, = ApiSession.issue_for(
    account: push_api_account, household_membership: push_api_membership, device_name: 'contract-push-api'
  )
  push_web_account, push_web_household, = create_household(nonce, 'push-web')
  push_observer_account, push_observer_household, = create_household(nonce, 'push-observer')
  push_observer_membership = push_observer_account.household_memberships.find_by!(household: push_observer_household)
  _push_observer_session, push_observer_access_token, = ApiSession.issue_for(
    account: push_observer_account, household_membership: push_observer_membership, device_name: 'contract-push-observer'
  )
  push_fatal_account, push_fatal_household, = create_household(nonce, 'push-fatal')
  push_fatal_membership = push_fatal_account.household_memberships.find_by!(household: push_fatal_household)
  _push_fatal_session, push_fatal_access_token, = ApiSession.issue_for(
    account: push_fatal_account, household_membership: push_fatal_membership, device_name: 'contract-push-fatal'
  )
  retained_account, retained_household, = create_household(nonce, 'retained')
  retained_location = Location.create!(household: retained_household, name: "Contract retained shelf #{nonce}")
  retained_medication = Medication.create!(household: retained_household, location: retained_location,
                                           name: "Contract retained medicine #{nonce}", dose_amount: '1',
                                           dose_unit: 'ml', current_supply: '50')
  retained_schedule = Schedule.create!(household: retained_household, person: retained_account.person,
                                       medication: retained_medication, dose_amount: '1', dose_unit: 'ml',
                                       frequency: 'Daily', start_date: '2026-01-01', end_date: '2099-12-31')
  web_ai_paid_account, web_ai_paid_household, = create_household(nonce, 'web-ai-paid')
  web_ai_paid_household.update!(subscription_plan: 'family_plus')
  web_ai_paid_member, = create_admin_member(web_ai_paid_household, nonce, 'web-ai-paid-member')
  web_ai_free_account, web_ai_free_household, = create_household(nonce, 'web-ai-free')
  web_ai_ip_account, web_ai_ip_household, = create_household(nonce, 'web-ai-ip')
  web_ai_ip_household.update!(subscription_plan: 'family_plus')
  web_ai_user_account, web_ai_user_household, = create_household(nonce, 'web-ai-user')
  web_ai_user_household.update!(subscription_plan: 'family_plus')
  web_people_account, web_people_household, = create_household(nonce, 'web-people')
  web_people_membership = web_people_account.household_memberships.find_by!(household: web_people_household)
  web_people_delete = web_people_household.people.create!(name: "Contract web delete #{nonce}",
                                                           date_of_birth: 30.years.ago.to_date,
                                                           person_type: :adult, has_capacity: true)
  web_people_history = web_people_household.people.create!(name: "Contract web history #{nonce}",
                                                            date_of_birth: 30.years.ago.to_date,
                                                            person_type: :adult, has_capacity: true)
  web_people_view_target = web_people_household.people.create!(name: "Contract web view target #{nonce}",
                                                                date_of_birth: 30.years.ago.to_date,
                                                                person_type: :adult, has_capacity: true)
  [web_people_delete, web_people_history, web_people_view_target].each do |person|
    PersonAccessGrant.create!(household: web_people_household, household_membership: web_people_membership,
                              person: person, access_level: :manage, relationship_type: :family_member,
                              granted_by_membership: web_people_membership)
  end
  web_people_member, = create_admin_member(web_people_household, nonce, 'web-people-member')
  web_people_view_member, = create_admin_member(web_people_household, nonce, 'web-people-view-member')
  PersonAccessGrant.create!(household: web_people_household, household_membership: web_people_view_member,
                            person: web_people_view_target, access_level: :view, relationship_type: :family_member,
                            granted_by_membership: web_people_membership)
  web_people_foreign_account, web_people_foreign_household, = create_household(nonce, 'web-people-foreign')
  web_people_foreign = web_people_foreign_account.person
  web_people_location = Location.create!(household: web_people_household, name: "Contract web shelf #{nonce}")
  web_people_medication = Medication.create!(household: web_people_household, location: web_people_location,
                                             name: "Contract web medicine #{nonce}", dose_amount: '1',
                                             dose_unit: 'ml', current_supply: '50')
  web_people_schedule = Schedule.create!(household: web_people_household, person: web_people_history,
                                         medication: web_people_medication, dose_amount: '1', dose_unit: 'ml',
                                         frequency: 'Daily', start_date: '2026-01-01', end_date: '2099-12-31')
  MedicationTake.create!(household: web_people_household, schedule: web_people_schedule,
                         taken_at: Time.current, dose_amount: '1', dose_unit: 'ml',
                         taken_from_medication: web_people_medication,
                         taken_from_location: web_people_location)
  portable_source_account, portable_source_household, = create_household(nonce, 'portable-source')
  portable_target_account, portable_target_household, = create_household(nonce, 'portable-target')
  profile_account, profile_household, = create_household(nonce, 'profile')
  avatar_account, avatar_household, = create_household(nonce, 'avatar')
  avatar_invalid_account, avatar_invalid_household, = create_household(nonce, 'avatar-invalid')
  web_avatar_account, web_avatar_household, = create_household(nonce, 'web-avatar')
  web_avatar_account.person.avatar.attach(io: StringIO.new('contract-owner-avatar'), filename: 'owner.png',
                                          content_type: 'image/png')
  web_avatar_hidden_person = web_avatar_household.people.create!(name: "Contract hidden avatar #{nonce}",
                                                                  date_of_birth: 25.years.ago.to_date,
                                                                  person_type: :adult, has_capacity: true)
  web_avatar_hidden_person.avatar.attach(io: StringIO.new('contract-hidden-avatar'), filename: 'hidden.png',
                                         content_type: 'image/png')
  avatar_membership = avatar_account.household_memberships.find_by!(household: avatar_household)
  _avatar_session, avatar_access_token, = ApiSession.issue_for(
    account: avatar_account, household_membership: avatar_membership, device_name: 'contract-avatar'
  )
  avatar_invalid_membership = avatar_invalid_account.household_memberships.find_by!(household: avatar_invalid_household)
  _avatar_invalid_session, avatar_invalid_access_token, = ApiSession.issue_for(
    account: avatar_invalid_account, household_membership: avatar_invalid_membership, device_name: 'contract-avatar-invalid'
  )
  profile_membership = profile_account.household_memberships.find_by!(household: profile_household)
  _profile_session, profile_access_token, = ApiSession.issue_for(
    account: profile_account, household_membership: profile_membership, device_name: 'contract-profile'
  )
  profile_view_membership, profile_view_access_token = create_admin_member(profile_household, nonce, 'profile-view')
  PersonAccessGrant.create!(household: profile_household, household_membership: profile_view_membership,
                            person: profile_view_membership.person, access_level: :view,
                            relationship_type: :family_member, granted_by_membership: profile_membership)
  PersonAccessGrant.create!(household: profile_household, household_membership: profile_view_membership,
                            person: profile_account.person, access_level: :manage,
                            relationship_type: :family_member, granted_by_membership: profile_membership)
  profile_revoke_membership, profile_revoke_access_token = create_admin_member(profile_household, nonce,
                                                                              'profile-revoke')
  profile_revoke_grant = PersonAccessGrant.create!(household: profile_household,
                                                  household_membership: profile_revoke_membership,
                                                  person: profile_revoke_membership.person, access_level: :manage,
                                                  relationship_type: :family_member,
                                                  granted_by_membership: profile_membership)
  profile_signed_blob = ActiveStorage::Blob.create_before_direct_upload!(
    filename: 'foreign.png', byte_size: 7, checksum: Digest::MD5.base64digest('foreign'),
    content_type: 'image/png'
  )
  upload_blob = File.open(Rails.root.join('public/icon.png')) do |image|
    ActiveStorage::Blob.create_and_upload!(io: image, filename: 'contract-icon.png', content_type: 'image/png')
  end
  lookup_paid_account, lookup_paid_household, = create_household(nonce, 'lookup-paid')
  lookup_paid_household.update!(subscription_plan: 'family_plus')
  lookup_paid_membership = lookup_paid_account.household_memberships.find_by!(household: lookup_paid_household)
  _lookup_paid_session, lookup_paid_access_token, = ApiSession.issue_for(
    account: lookup_paid_account, household_membership: lookup_paid_membership, device_name: 'contract-lookup-paid'
  )
  lookup_barcode = '9876543210987'
  lookup_hidden_barcode = '9876543210988'
  web_foreign_barcode = '9876543210997'
  web_foreign_display = "Contract foreign catalogue medicine #{nonce}"
  lookup_display = "Contract catalogue medicine #{nonce}"
  lookup_code = "contract-dmd-#{nonce}"
  BarcodeCatalogEntry.create!(gtin: lookup_barcode, display: lookup_display, source: 'contract_catalog',
                              code: lookup_code, system: 'https://dmd.nhs.uk', concept_class: 'AMPP')
  BarcodeCatalogEntry.create!(gtin: lookup_hidden_barcode, display: "Contract hidden catalogue medicine #{nonce}",
                              source: 'contract_catalog', code: "contract-hidden-dmd-#{nonce}",
                              system: 'https://dmd.nhs.uk', concept_class: 'AMPP')
  BarcodeCatalogEntry.create!(gtin: web_foreign_barcode, display: web_foreign_display,
                              source: 'contract_catalog', code: "contract-web-foreign-#{nonce}",
                              system: 'https://dmd.nhs.uk', concept_class: 'AMPP')
  portable_source_membership = portable_source_account.household_memberships.find_by!(household: portable_source_household)
  portable_target_membership = portable_target_account.household_memberships.find_by!(household: portable_target_household)
  _portable_source_session, portable_source_access_token, = ApiSession.issue_for(
    account: portable_source_account, household_membership: portable_source_membership,
    device_name: 'contract-portable-source'
  )
  _portable_target_session, portable_target_access_token, = ApiSession.issue_for(
    account: portable_target_account, household_membership: portable_target_membership,
    device_name: 'contract-portable-target'
  )
  _portable_target_app, portable_target_app_token = ApiAppToken.issue_for(
    account: portable_target_account, household_membership: portable_target_membership,
    name: 'Contract portable target app'
  )
  _portable_member, portable_member_access_token = create_admin_member(portable_target_household, nonce,
                                                                        'portable-member')
  portable_revoked_membership, portable_revoked_access_token = create_admin_member(
    portable_target_household, nonce, 'portable-revoked'
  )
  portable_revoked_membership.update!(status: :revoked)
  portable_locked_membership, portable_locked_access_token = create_admin_member(
    portable_target_household, nonce, 'portable-locked'
  )
  AccountLockout.create!(account_id: portable_locked_membership.account_id, key: SecureRandom.hex(16),
                         deadline: 30.minutes.from_now)
  portable_source_person = portable_source_household.people.create!(name: "Contract portable patient #{nonce}",
                                                                     date_of_birth: '1990-01-01',
                                                                     person_type: :adult, has_capacity: true)
  PersonAccessGrant.create!(household: portable_source_household,
                            household_membership: portable_source_membership, person: portable_source_person,
                            access_level: :manage, relationship_type: :family_member,
                            granted_by_membership: portable_source_membership)
  portable_source_location = Location.create!(household: portable_source_household,
                                              name: "Contract portable shelf #{nonce}")
  LocationMembership.create!(household: portable_source_household, person: portable_source_person,
                             location: portable_source_location)
  portable_source_medication = Medication.create!(household: portable_source_household,
                                                  location: portable_source_location,
                                                  name: "Contract portable medicine #{nonce}", current_supply: '20')
  portable_source_dosage = portable_source_medication.dosage_records.create!(amount: '2', unit: 'ml',
                                                                              frequency: 'Daily',
                                                                              default_max_daily_doses: 4,
                                                                              default_min_hours_between_doses: '4',
                                                                              default_dose_cycle: :daily)
  portable_source_schedule = Schedule.create!(household: portable_source_household,
                                              person: portable_source_person, medication: portable_source_medication,
                                              source_dosage_option: portable_source_dosage,
                                              dose_amount: '2', dose_unit: 'ml', dose_cycle: :daily, frequency: 'Daily',
                                              start_date: '2026-01-01', end_date: '2099-12-31')
  portable_source_assignment = PersonMedication.create!(household: portable_source_household,
                                                        person: portable_source_person,
                                                        medication: portable_source_medication,
                                                        source_dosage_option: portable_source_dosage,
                                                        administration_kind: :as_needed,
                                                        dose_amount: '2', dose_unit: 'ml', dose_cycle: :daily)
  MedicationTake.create!(household: portable_source_household, schedule: portable_source_schedule,
                         taken_at: Time.zone.parse('2026-02-25 12:00:00'), dose_amount: '2', dose_unit: 'ml',
                         taken_from_medication: portable_source_medication,
                         taken_from_location: portable_source_location)
  MedicationTake.create!(household: portable_source_household, person_medication: portable_source_assignment,
                         taken_at: Time.zone.parse('2026-02-26 12:00:00'), dose_amount: '2', dose_unit: 'ml',
                         taken_from_medication: portable_source_medication,
                         taken_from_location: portable_source_location)
  NotificationPreference.create!(household: portable_source_household, person: portable_source_person,
                                 enabled: true)
  portable_review_partner = Medication.create!(household: portable_source_household,
                                               location: portable_source_location,
                                               name: "Contract portable review partner #{nonce}",
                                               dose_amount: '1', dose_unit: 'tablet')
  portable_review_evidence = MedicationReviewEvidenceRecord.create!(source_name: 'Contract source',
                                                                     source_record_id: "contract-portable-#{nonce}",
                                                                     source_url: 'https://example.test/evidence',
                                                                     retrieved_on: '2026-02-25',
                                                                     product_name: 'Contract medicine',
                                                                     label_section: 'warnings',
                                                                     evidence_text: 'Contract reviewed pair',
                                                                     risk_level: 'high', match_confidence: 'high',
                                                                     match_status: 'not_pairwise')
  MedicationReviewPrompt.create!(household: portable_source_household, person: portable_source_person,
                                 primary_medication: portable_source_medication,
                                 interacting_medication: portable_review_partner,
                                 evidence_record: portable_review_evidence, risk_level: 'high',
                                 match_confidence: 'high', primary_medication_name: portable_source_medication.name,
                                 interacting_medication_name: portable_review_partner.name,
                                 evidence_source_name: portable_review_evidence.source_name,
                                 evidence_source_url: portable_review_evidence.source_url,
                                 evidence_source_checked_on: portable_review_evidence.retrieved_on,
                                 evidence_source_version: 'contract-v1',
                                 evidence_source_effective_on: portable_review_evidence.retrieved_on,
                                 matched_term: 'Contract medicine', match_type: 'reviewed_pair',
                                 source_instruction: 'Discuss with a practitioner',
                                 match_reason: 'Contract pair', evidence_text: portable_review_evidence.evidence_text,
                                 status: 'needs_review')
  portable_source_household.locations.find_each do |location|
    location.update!(name: "Contract portable source #{location.name} #{nonce}")
  end
  portable_payload = PortableData::Exporter.new(household: portable_source_household,
                                                membership: portable_source_membership,
                                                passphrase: 'contract portable secret').payload
  portable_numeric_payload = portable_payload.deep_dup
  portable_numeric_payload[:records][:people].first[:id] = portable_source_person.id
  portable_conflict_payload = portable_payload.deep_dup
  portable_target_location = Location.create!(household: portable_target_household,
                                              name: "Contract portable target shelf #{nonce}")
  portable_conflict_payload[:records][:locations].first[:name] = portable_target_location.name
  portable_malformed_payload = portable_payload.deep_dup
  portable_malformed_payload[:records][:people] = 'invalid collection'
  foreign_membership = foreign_account.household_memberships.find_by!(household: foreign_household)
  foreign_app_token, foreign_app_token_raw = ApiAppToken.issue_for(
    account: foreign_account, household_membership: foreign_membership, name: 'Contract foreign app token'
  )
  _foreign_session, foreign_access_token, = ApiSession.issue_for(
    account: foreign_account, household_membership: foreign_membership, device_name: 'contract-foreign'
  )
  membership = account.household_memberships.find_by!(household: household)
  manager_membership, manager_access_token = create_admin_member(household, nonce, 'manager', role: :administrator)
  invitation_authority_membership, invitation_authority_access_token = create_admin_member(
    household, nonce, 'invitation-authority', role: :administrator
  )
  token_authority_membership, token_authority_access_token = create_admin_member(household, nonce, 'token-authority',
                                                                                  role: :administrator)
  manager_app_token, manager_app_token_value = ApiAppToken.issue_for(account: manager_membership.account,
                                                                     household_membership: manager_membership,
                                                                     name: 'Contract manager app token')
  invitation_accept_account, invitation_accept_household, = create_household(nonce, 'invitation-accept')
  invitation_accept_membership = invitation_accept_account.household_memberships.find_by!(
    household: invitation_accept_household
  )
  _invitation_accept_session, invitation_accept_access_token, = ApiSession.issue_for(
    account: invitation_accept_account, household_membership: invitation_accept_membership,
    device_name: 'contract-invitation-accept'
  )
  invitation_expired_account, invitation_expired_household, = create_household(nonce, 'invitation-expired')
  invitation_expired_membership = invitation_expired_account.household_memberships.find_by!(
    household: invitation_expired_household
  )
  _invitation_expired_session, invitation_expired_access_token, = ApiSession.issue_for(
    account: invitation_expired_account, household_membership: invitation_expired_membership,
    device_name: 'contract-invitation-expired'
  )
  invitation_revoked_account, invitation_revoked_household, = create_household(nonce, 'invitation-revoked')
  invitation_revoked_membership = invitation_revoked_account.household_memberships.find_by!(
    household: invitation_revoked_household
  )
  _invitation_revoked_session, invitation_revoked_access_token, = ApiSession.issue_for(
    account: invitation_revoked_account, household_membership: invitation_revoked_membership,
    device_name: 'contract-invitation-revoked'
  )
  invitation_rotation_account, invitation_rotation_household, = create_household(nonce, 'invitation-rotation')
  invitation_rotation_membership = invitation_rotation_account.household_memberships.find_by!(
    household: invitation_rotation_household
  )
  _invitation_rotation_session, invitation_rotation_access_token, = ApiSession.issue_for(
    account: invitation_rotation_account, household_membership: invitation_rotation_membership,
    device_name: 'contract-invitation-rotation'
  )
  invitation_mobile_account, = create_household(nonce, 'invitation-mobile')
  revoked_owner_app_token, = ApiAppToken.issue_for(account: account, household_membership: membership,
                                                   name: 'Contract revoked owner app token')
  revoked_owner_app_token.revoke!
  admin_target_membership, admin_target_access_token = create_admin_member(household, nonce, 'admin-target')
  admin_owner_patch_membership, = create_admin_member(household, nonce, 'admin-owner-patch')
  admin_manager_put_membership, admin_manager_put_access_token = create_admin_member(household, nonce, 'admin-manager-put')
  admin_owner_put_membership, admin_owner_put_access_token = create_admin_member(household, nonce, 'admin-owner-put')
  admin_invalid_membership, = create_admin_member(household, nonce, 'admin-invalid')
  admin_revoke_membership, admin_revoke_access_token = create_admin_member(household, nonce, 'admin-revoke')
  last_owner_account, last_owner_household, = create_household(nonce, 'last-owner')
  last_owner_membership = last_owner_account.household_memberships.find_by!(household: last_owner_household)
  _last_owner_session, last_owner_access_token, = ApiSession.issue_for(
    account: last_owner_account, household_membership: last_owner_membership, device_name: 'contract-last-owner'
  )
  managed_person = household.people.create!(name: "Contract managed #{nonce}", date_of_birth: 35.years.ago.to_date,
                                            person_type: :adult, has_capacity: true)
  hidden_person = household.people.create!(name: "Contract hidden #{nonce}", date_of_birth: 36.years.ago.to_date,
                                           person_type: :adult, has_capacity: true)
  PersonAccessGrant.create!(household: household, household_membership: membership, person: managed_person,
                            access_level: :manage, relationship_type: :family_member,
                            granted_by_membership: membership)
  invitation_accept = household.household_invitations.create!(
    email: invitation_accept_account.email, membership_role: :member, invited_by_membership: membership
  )
  invitation_accept.household_invitation_grants.create!(
    household: household, person: managed_person, access_level: :record, relationship_type: :professional
  )
  invitation_expired = household.household_invitations.create!(
    email: invitation_expired_account.email, membership_role: :member,
    invited_by_membership: membership, expires_at: 1.day.ago
  )
  invitation_expired.household_invitation_grants.create!(
    household: household, person: managed_person, access_level: :record, relationship_type: :professional
  )
  invitation_revoked = household.household_invitations.create!(
    email: invitation_revoked_account.email, membership_role: :member, invited_by_membership: membership
  )
  invitation_rotation = household.household_invitations.create!(
    email: invitation_rotation_account.email, membership_role: :member, invited_by_membership: membership
  )
  invitation_mobile = household.household_invitations.create!(
    email: invitation_mobile_account.email, membership_role: :member, invited_by_membership: membership
  )
  invitation_duplicate_email = "contract-invitation-duplicate-#{nonce}@example.test"
  household.household_invitations.create!(
    email: invitation_duplicate_email, membership_role: :member, invited_by_membership: membership
  )
  invitation_authority = household.household_invitations.create!(
    email: "contract-invitation-authority-#{nonce}@example.test", membership_role: :member,
    invited_by_membership: invitation_authority_membership
  )
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
  feed_account = Account.create!(email: "contract-feed-#{nonce}@example.test", status: :verified,
                                 password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  feed_person = household.people.create!(account: feed_account, name: "Contract feed actor #{nonce}",
                                         date_of_birth: 34.years.ago.to_date, person_type: :adult, has_capacity: true)
  User.create!(person: feed_person, email_address: feed_account.email, password: 'password', active: true)
  feed_membership = household.household_memberships.create!(account: feed_account, person: feed_person,
                                                            role: :owner, status: :active)
  PersonAccessGrant.create!(household: household, household_membership: feed_membership, person: hidden_person,
                            access_level: :manage, relationship_type: :family_member,
                            granted_by_membership: membership)
  _feed_session, feed_access_token, = ApiSession.issue_for(
    account: feed_account, household_membership: feed_membership, device_name: 'contract-feed'
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
  replay_account = Account.create!(email: "contract-replay-#{nonce}@example.test", status: :verified,
                                   password_hash: RodauthApp.rodauth.allocate.password_hash('password'))
  replay_person = household.people.create!(account: replay_account, name: "Contract replay #{nonce}",
                                           date_of_birth: 30.years.ago.to_date, person_type: :adult, has_capacity: true)
  User.create!(person: replay_person, email_address: replay_account.email, password: 'password', active: true)
  replay_membership = household.household_memberships.create!(account: replay_account, person: replay_person,
                                                              role: :member, status: :active)
  replay_grant = PersonAccessGrant.create!(household: household, household_membership: replay_membership,
                                          person: managed_person, access_level: :manage,
                                          relationship_type: :carer, granted_by_membership: membership)
  _replay_session, replay_access_token, = ApiSession.issue_for(
    account: replay_account, household_membership: replay_membership, device_name: 'contract-replay'
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
  hidden_location = Location.create!(household: household, name: "Contract hidden shelf #{nonce}")
  LocationMembership.create!(household: household, person: hidden_person, location: hidden_location)
  foreign_location = Location.create!(household: foreign_household, name: "Contract foreign shelf #{nonce}")
  managed_medication = Medication.create!(household: household, location: primary_location,
                                          name: "Contract managed medicine #{nonce}", dose_amount: '2',
                                          dose_unit: 'ml', current_supply: '50', reorder_threshold: '5',
                                          category: 'Analgesic', dmd_code: '123456', dmd_system: 'https://dmd.nhs.uk')
  managed_medication.update!(barcode: lookup_barcode)
  managed_dosage = managed_medication.dosage_records.create!(amount: '1', unit: 'ml', frequency: 'daily',
                                                             default_max_daily_doses: 4,
                                                             default_min_hours_between_doses: '4',
                                                             default_dose_cycle: :daily)
  hidden_medication = Medication.create!(household: household, location: primary_location,
                                         name: "Contract hidden medicine #{nonce}", dose_amount: '2',
                                         dose_unit: 'ml', current_supply: '50', reorder_threshold: '5', category: 'Vitamin')
  hidden_medication.update!(barcode: lookup_hidden_barcode)
  foreign_medication = Medication.create!(household: foreign_household, location: foreign_location,
                                          name: "Contract foreign medicine #{nonce}", dose_amount: '2',
                                          dose_unit: 'ml', current_supply: '50', reorder_threshold: '5')
  foreign_medication.update!(barcode: web_foreign_barcode)
  foreign_dosage = foreign_medication.dosage_records.create!(amount: '1', unit: 'ml', frequency: 'daily',
                                                             default_max_daily_doses: 4,
                                                             default_min_hours_between_doses: '4',
                                                             default_dose_cycle: :daily)
  managed_assignment = PersonMedication.create!(household: household, person: managed_person,
                                                medication: managed_medication, administration_kind: :as_needed)
  managed_assignment_updated_at = Time.utc(2026, 1, 1)
  managed_assignment.update_columns(updated_at: managed_assignment_updated_at)
  retired_medication = Medication.create!(household: household, location: primary_location,
                                          name: "Contract retired medicine #{nonce}", dose_amount: '1',
                                          dose_unit: 'ml')
  retired_assignment = PersonMedication.create!(household: household, person: managed_person,
                                                medication: retired_medication, administration_kind: :as_needed)
  retired_assignment.pause!
  retired_assignment_period = retired_assignment.medication_pause_periods.sole
  retired_assignment.retire!
  hidden_assignment = PersonMedication.create!(household: household, person: hidden_person,
                                               medication: hidden_medication, administration_kind: :as_needed)
  foreign_assignment = PersonMedication.create!(household: foreign_household, person: foreign_account.person,
                                                medication: foreign_medication, administration_kind: :as_needed)
  visible_low_stock = Medication.create!(household: household, location: primary_location,
                                         name: "Contract visible low stock #{nonce}", current_supply: '1',
                                         reorder_threshold: '5')
  hidden_low_stock = Medication.create!(household: household, location: hidden_location,
                                        name: "Contract hidden low stock #{nonce}", current_supply: '1',
                                        reorder_threshold: '5')
  foreign_low_stock = Medication.create!(household: foreign_household, location: foreign_location,
                                         name: "Contract foreign low stock #{nonce}", current_supply: '1',
                                         reorder_threshold: '5')
  PersonMedication.create!(household: household, person: managed_person, medication: visible_low_stock,
                           administration_kind: :as_needed, dose_amount: '1', dose_unit: 'tablet')
  PersonMedication.create!(household: household, person: hidden_person, medication: hidden_low_stock,
                           administration_kind: :as_needed, dose_amount: '1', dose_unit: 'tablet')
  PersonMedication.create!(household: foreign_household, person: foreign_account.person,
                           medication: foreign_low_stock, administration_kind: :as_needed,
                           dose_amount: '1', dose_unit: 'tablet')
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
  managed_health_event = HealthEvent.create!(household: household, person: managed_person, event_kind: :illness,
                                            title: "Contract managed event #{nonce}", started_on: '2026-02-25')
  HealthEvent.create!(household: household, person: managed_person, event_kind: :illness,
                      title: managed_health_event.title, started_on: '2026-02-26')
  managed_side_effect = HealthEvent.create!(household: household, person: managed_person,
                                            event_kind: :suspected_side_effect,
                                            title: "Contract managed side effect #{nonce}", started_on: '2026-02-25')
  earlier_health_event = HealthEvent.create!(household: household, person: managed_person, event_kind: :illness,
                                            title: "Contract earlier event #{nonce}", started_on: '2026-02-20',
                                            ended_on: '2026-02-21')
  HealthEventMedication.create!(household: household, health_event: managed_health_event,
                                medication: managed_medication)
  historical_location = Location.create!(household: household, name: "Contract historical shelf #{nonce}")
  historical_medication = Medication.create!(household: household, location: historical_location,
                                             name: "Contract historical medicine #{nonce}", dose_amount: '1',
                                             dose_unit: 'ml', current_supply: '5')
  fhir_stopped_schedule = Schedule.create!(household: household, person: managed_person,
                                           medication: retired_medication, dose_amount: '1', dose_unit: 'ml',
                                           frequency: 'Daily', start_date: '2026-02-25', end_date: '2099-12-31')
  fhir_stopped_schedule.pause!
  fhir_stopped_assignment = PersonMedication.create!(household: household, person: managed_person,
                                                      medication: historical_medication, administration_kind: :as_needed)
  fhir_stopped_assignment.pause!
  historical_schedule = Schedule.create!(household: household, person: managed_person,
                                         medication: historical_medication, dose_amount: '1', dose_unit: 'ml',
                                         frequency: 'Daily', start_date: '2026-02-25', end_date: '2099-12-31')
  managed_occurrence = historical_schedule.medication_dose_occurrences.create!(window_starts_on: Date.current,
                                                                               position: 1, outcome: 'open')
  hidden_schedule = Schedule.create!(household: household, person: hidden_person, medication: hidden_medication,
                                     dose_amount: '1', dose_unit: 'ml', frequency: 'Daily',
                                     start_date: '2026-02-25', end_date: '2099-12-31')
  foreign_schedule = Schedule.create!(household: foreign_household, person: foreign_account.person,
                                      medication: foreign_medication, dose_amount: '1', dose_unit: 'ml',
                                      frequency: 'Daily', start_date: '2026-02-25', end_date: '2099-12-31')
  managed_take = MedicationTake.create!(household: household, schedule: historical_schedule, taken_at: Time.current,
                                       dose_amount: '1', dose_unit: 'ml', taken_from_medication: historical_medication,
                                       taken_from_location: historical_location)
  MedicationTake.create!(household: household, schedule: historical_schedule,
                         taken_at: Time.zone.parse('2026-02-25 12:00:00'), dose_amount: '1', dose_unit: 'ml',
                         taken_from_medication: historical_medication, taken_from_location: historical_location)
  hidden_take = MedicationTake.create!(household: household, schedule: hidden_schedule, taken_at: Time.current,
                                      dose_amount: '1', dose_unit: 'ml', taken_from_medication: hidden_medication,
                                      taken_from_location: primary_location, client_uuid: SecureRandom.uuid)
  foreign_take = MedicationTake.create!(household: foreign_household, schedule: foreign_schedule,
                                       taken_at: Time.current, dose_amount: '1', dose_unit: 'ml',
                                       taken_from_medication: foreign_medication,
                                       taken_from_location: foreign_location)
  managed_preference = NotificationPreference.create!(household: household, person: managed_person, enabled: true)
  hidden_preference = NotificationPreference.create!(household: household, person: hidden_person, enabled: true)
  foreign_preference = NotificationPreference.create!(household: foreign_household,
                                                      person: foreign_account.person, enabled: true)
  hidden_health_event = HealthEvent.create!(household: household, person: hidden_person, event_kind: :illness,
                                            title: "Contract hidden event #{nonce}", started_on: '2026-02-25')
  HealthEvent.create!(household: household, person: hidden_person, event_kind: :illness,
                      title: hidden_health_event.title, started_on: '2026-02-26')
  HealthEvent.create!(household: household, person: hidden_person, event_kind: :suspected_side_effect,
                      title: "Contract hidden side effect #{nonce}", started_on: '2026-02-25')
  cursor_boundary_at = Time.utc(2026, 1, 1)
  cursor_boundary_location = nil
  TenantContext.with(account: feed_account, household: household, membership: feed_membership,
                     request_id: "contract-feed-#{nonce}") do
    cursor_boundary_location = Location.create!(household: household, name: "Contract cursor boundary #{nonce}")
    cursor_boundary_location.destroy!
  end
  cursor_boundary_location_portable_id = cursor_boundary_location.portable_id
  ApiChangeEvent.find_by!(household: household, record_type: 'Location',
                          record_portable_id: cursor_boundary_location_portable_id)
                .update_columns(occurred_at: cursor_boundary_at)
  ApiTombstone.find_by!(household: household, record_type: 'Location',
                        record_portable_id: cursor_boundary_location_portable_id)
              .update_columns(deleted_at: cursor_boundary_at)
  foreign_health_event = HealthEvent.create!(household: foreign_household, person: foreign_account.person,
                                             event_kind: :illness, title: "Contract foreign event #{nonce}",
                                             started_on: '2026-02-25')
  HealthEvent.create!(household: foreign_household, person: foreign_account.person, event_kind: :illness,
                      title: foreign_health_event.title, started_on: '2026-02-26')
  HealthEvent.create!(household: foreign_household, person: foreign_account.person,
                      event_kind: :suspected_side_effect,
                      title: "Contract foreign side effect #{nonce}", started_on: '2026-02-25')
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
  100.times do |index|
    evidence = review_evidence.fetch('high').dup
    evidence.source_record_id = "contract-review-page-#{index}-#{nonce}"
    evidence.save!
    review_attributes.call(foreign_household, foreign_account.person, foreign_medication,
                           foreign_review_partner, evidence, 'needs_review')
  end
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

  deactivated_account, deactivated_household, deactivated_user = create_household(nonce, 'auth-deactivated')
  deactivated_membership = deactivated_account.household_memberships.active.sole
  _deactivated_session, deactivated_access_token, = ApiSession.issue_for(
    account: deactivated_account, household_membership: deactivated_membership, device_name: 'contract-deactivated'
  )
  _deactivated_app_token, deactivated_app_token = ApiAppToken.issue_for(
    account: deactivated_account, household_membership: deactivated_membership, name: 'Contract deactivated'
  )
  deactivated_user.deactivate!

  inactive_account, inactive_household, inactive_user = create_household(nonce, 'auth-inactive')
  inactive_membership = inactive_account.household_memberships.active.sole
  _inactive_session, inactive_access_token, = ApiSession.issue_for(
    account: inactive_account, household_membership: inactive_membership, device_name: 'contract-inactive'
  )
  inactive_user.update!(active: false)

  operational_states = %i[held offboarded purged].to_h do |state|
    operational_account, operational_household, = create_household(nonce, "auth-#{state}")
    operational_membership = operational_account.household_memberships.active.sole
    _operational_session, operational_token, = ApiSession.issue_for(
      account: operational_account, household_membership: operational_membership, device_name: "contract-#{state}"
    )
    operational_household.update!(lifecycle_state: state)
    [state, { household_id: operational_household.id, access_token: operational_token }]
  end

  suspended_account, suspended_household, = create_household(nonce, 'auth-suspended')
  suspended_membership, suspended_access_token = create_admin_member(suspended_household, nonce, 'auth-suspended-member')
  suspended_membership.update!(status: :suspended)

  role_account, role_household, = create_household(nonce, 'auth-role-change')
  role_owner_membership = role_account.household_memberships.active.sole
  _role_owner_session, role_owner_access_token, = ApiSession.issue_for(
    account: role_account, household_membership: role_owner_membership, device_name: 'contract-role-owner'
  )
  role_member_membership, role_member_access_token = create_admin_member(role_household, nonce, 'auth-role-member')
  _role_member_app, role_member_app_token = ApiAppToken.issue_for(
    account: role_member_membership.account, household_membership: role_member_membership,
    name: 'Contract role member'
  )

  oauth_client_id = "contract-mobile-#{nonce}"
  oauth_redirect_uri = 'io.damacus.medtracker.contract:/oauth2redirect'
  oauth_application = OauthApplication.create!(name: 'Contract mobile', client_id: oauth_client_id, client_kind: :mobile,
                                               redirect_uri: oauth_redirect_uri, scopes: 'medtracker offline_access',
                                               token_endpoint_auth_method: 'none')
  profile_revoke_mobile_token = "contract-profile-revoke-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: profile_revoke_membership.account, oauth_application: oauth_application,
                     client_kind: :mobile, scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(profile_revoke_mobile_token))
  portable_target_mobile_token = "contract-portable-target-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: portable_target_account, oauth_application: oauth_application, client_kind: :mobile,
                     scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(portable_target_mobile_token))
  care_access_token = "contract-care-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: care_account, oauth_application: oauth_application, client_kind: :mobile,
                     scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(care_access_token))
  replay_mobile_access_token = "contract-replay-mobile-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: replay_account, oauth_application: oauth_application, client_kind: :mobile,
                     scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(replay_mobile_access_token))
  invitation_mobile_oauth_token = "contract-invitation-mobile-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: invitation_mobile_account, oauth_application: oauth_application, client_kind: :mobile,
                     scopes: 'medtracker offline_access', expires_in: 1.hour.from_now,
                     authenticated_at: Time.current, last_used_at: Time.current,
                     token_hash: OauthGrant.digest(invitation_mobile_oauth_token))

  fhir_application = OauthApplication.create!(name: 'Contract SMART FHIR', client_id: SecureRandom.uuid,
                                              redirect_uri: 'https://client.example/callback',
                                              scopes: 'patient/Patient.rs patient/Medication.rs',
                                              token_endpoint_auth_method: 'none')
  role_member_oauth_token = "contract-auth-role-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: role_member_membership.account, oauth_application: fhir_application,
                     household_membership: role_member_membership, person: role_member_membership.person,
                     permissions_version: role_member_membership.permissions_version,
                     token_hash: OauthGrant.digest(role_member_oauth_token),
                     expires_in: 1.hour.from_now, scopes: 'patient/Patient.rs')
  fhir_patient_scope_token = "contract-fhir-patient-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: account, oauth_application: fhir_application,
                     household_membership: membership, person: managed_person,
                     permissions_version: membership.permissions_version,
                     token_hash: OauthGrant.digest(fhir_patient_scope_token),
                     expires_in: 1.hour.from_now, scopes: 'patient/Patient.rs')
  fhir_revoked_scope_token = "contract-fhir-revoked-#{SecureRandom.urlsafe_base64(48)}"
  OauthGrant.create!(account: account, oauth_application: fhir_application,
                     household_membership: membership, person: managed_person,
                     permissions_version: membership.permissions_version,
                     token_hash: OauthGrant.digest(fhir_revoked_scope_token),
                     expires_in: 1.hour.from_now, scopes: 'patient/Patient.rs', revoked_at: Time.current)

  {
    web_device_household_id: web_device_household.id,
    web_device_household_slug: web_device_household.slug,
    web_device_email: web_device_account.email,
    web_device_access_token: web_device_access_token,
    push_api_household_id: push_api_household.id,
    push_api_access_token: push_api_access_token,
    push_web_household_id: push_web_household.id,
    push_web_household_slug: push_web_household.slug,
    push_web_email: push_web_account.email,
    push_observer_household_id: push_observer_household.id,
    push_observer_access_token: push_observer_access_token,
    push_fatal_household_id: push_fatal_household.id,
    push_fatal_household_slug: push_fatal_household.slug,
    push_fatal_email: push_fatal_account.email,
    push_fatal_access_token: push_fatal_access_token,
    retained_household_slug: retained_household.slug,
    retained_household_id: retained_household.id,
    retained_email: retained_account.email,
    retained_schedule_id: retained_schedule.id,
    retained_medication_id: retained_medication.id,
    retained_foreign_household_slug: foreign_household.slug,
    web_household_slug: household.slug,
    web_foreign_barcode: web_foreign_barcode,
    web_foreign_display: web_foreign_display,
    web_view_email: view_account.email,
    web_feed_email: feed_account.email,
    web_managed_person_name: managed_person.name,
    web_hidden_person_name: hidden_person.name,
    web_ai_paid_slug: web_ai_paid_household.slug,
    web_ai_paid_email: web_ai_paid_account.email,
    web_ai_paid_member_email: web_ai_paid_member.account.email,
    web_ai_free_slug: web_ai_free_household.slug,
    web_ai_free_email: web_ai_free_account.email,
    web_ai_ip_slug: web_ai_ip_household.slug,
    web_ai_ip_email: web_ai_ip_account.email,
    web_ai_user_slug: web_ai_user_household.slug,
    web_ai_user_email: web_ai_user_account.email,
    web_people_slug: web_people_household.slug,
    web_people_email: web_people_account.email,
    web_people_delete_id: web_people_delete.id,
    web_people_view_target_id: web_people_view_target.id,
    web_people_history_id: web_people_history.id,
    web_people_history_schedule_id: web_people_schedule.id,
    web_people_foreign_slug: web_people_foreign_household.slug,
    web_people_foreign_email: web_people_foreign_account.email,
    web_people_foreign_id: web_people_foreign.id,
    web_people_member_email: web_people_member.account.email,
    web_people_view_member_email: web_people_view_member.account.email,
    profile_household_id: profile_household.id,
    avatar_household_id: avatar_household.id,
    web_avatar_household_slug: web_avatar_household.slug,
    web_avatar_email: web_avatar_account.email,
    web_avatar_person_id: web_avatar_account.person.id,
    web_avatar_hidden_person_id: web_avatar_hidden_person.id,
    avatar_invalid_household_slug: avatar_invalid_household.slug,
    avatar_access_token: avatar_access_token,
    avatar_invalid_household_id: avatar_invalid_household.id,
    avatar_invalid_access_token: avatar_invalid_access_token,
    profile_account_id: profile_account.id,
    profile_email: profile_account.email,
    profile_person_id: profile_account.person.id,
    profile_access_token: profile_access_token,
    profile_view_account_id: profile_view_membership.account_id,
    profile_view_person_id: profile_view_membership.person_id,
    profile_view_access_token: profile_view_access_token,
    profile_revoke_person_id: profile_revoke_membership.person_id,
    profile_revoke_membership_id: profile_revoke_membership.id,
    profile_revoke_grant_id: profile_revoke_grant.id,
    profile_revoke_access_token: profile_revoke_access_token,
    profile_revoke_mobile_token: profile_revoke_mobile_token,
    profile_signed_blob_id: profile_signed_blob.signed_id,
    upload_blob_signed_id: upload_blob.signed_id,
    upload_variation_key: upload_blob.variant(resize_to_limit: [16, 16]).variation.key,
    lookup_paid_household_id: lookup_paid_household.id,
    lookup_paid_account_id: lookup_paid_account.id,
    lookup_paid_membership_id: lookup_paid_membership.id,
    lookup_paid_access_token: lookup_paid_access_token,
    lookup_barcode: lookup_barcode,
    lookup_hidden_barcode: lookup_hidden_barcode,
    lookup_code: lookup_code,
    lookup_display: lookup_display,
    portable_source_household_id: portable_source_household.id,
    portable_source_account_id: portable_source_account.id,
    portable_source_membership_id: portable_source_membership.id,
    portable_source_access_token: portable_source_access_token,
    portable_source_person_name: portable_source_person.name,
    portable_source_person_portable_id: portable_source_person.portable_id,
    portable_source_location_portable_id: portable_source_location.portable_id,
    portable_source_medication_portable_id: portable_source_medication.portable_id,
    portable_source_schedule_portable_id: portable_source_schedule.portable_id,
    portable_target_household_id: portable_target_household.id,
    portable_target_account_id: portable_target_account.id,
    portable_target_membership_id: portable_target_membership.id,
    portable_target_access_token: portable_target_access_token,
    portable_target_app_token: portable_target_app_token,
    portable_target_mobile_token: portable_target_mobile_token,
    portable_member_access_token: portable_member_access_token,
    portable_revoked_access_token: portable_revoked_access_token,
    portable_locked_access_token: portable_locked_access_token,
    portable_numeric_bundle: PortableData::Encryptor.encrypt(portable_numeric_payload,
                                                              passphrase: 'contract portable secret'),
    portable_conflict_bundle: PortableData::Encryptor.encrypt(portable_conflict_payload,
                                                               passphrase: 'contract portable secret'),
    portable_malformed_bundle: PortableData::Encryptor.encrypt(portable_malformed_payload,
                                                                passphrase: 'contract portable secret'),
    access_token: access_token,
    account_id: account.id,
    platform_admin_email: platform_account.email,
    platform_target_email: platform_target_membership.account.email,
    platform_target_user_id: platform_target_membership.person.user.id,
    platform_promote_membership_id: platform_promote_membership.id,
    platform_promote_email: platform_promote_membership.account.email,
    platform_denied_user_id: platform_denied_membership.person.user.id,
    platform_denied_membership_id: platform_denied_membership.id,
    platform_denied_email: platform_denied_membership.account.email,
    platform_household_id: platform_household.id,
    platform_support_household_id: platform_support_household.id,
    platform_support_household_slug: platform_support_household.slug,
    platform_support_audit_token: platform_support_audit_token,
    platform_unrelated_household_slug: foreign_household.slug,
    primary_email: account.email,
    user_id: user.id,
    household_id: household.id,
    household_slug: household.slug,
    household_name: household.name,
    owner_membership_id: membership.id,
    revoked_owner_app_token_id: revoked_owner_app_token.id,
    foreign_household_id: foreign_household.id,
    foreign_household_slug: foreign_household.slug,
    foreign_membership_id: foreign_membership.id,
    foreign_app_token_id: foreign_app_token.id,
    foreign_app_token: foreign_app_token_raw,
    foreign_access_token: foreign_access_token,
    fhir_patient_scope_token: fhir_patient_scope_token,
    fhir_revoked_scope_token: fhir_revoked_scope_token,
    foreign_email: "contract-foreign-#{nonce}@example.test",
    manager_membership_id: manager_membership.id,
    manager_access_token: manager_access_token,
    invitation_authority_membership_id: invitation_authority_membership.id,
    invitation_authority_access_token: invitation_authority_access_token,
    invitation_authority_id: invitation_authority.id,
    token_authority_membership_id: token_authority_membership.id,
    token_authority_access_token: token_authority_access_token,
    manager_app_token_id: manager_app_token.id,
    manager_app_token: manager_app_token_value,
    invitation_accept_id: invitation_accept.id,
    invitation_accept_email: invitation_accept_account.email,
    invitation_accept_account_id: invitation_accept_account.id,
    invitation_accept_access_token: invitation_accept_access_token,
    invitation_accept_token: invitation_accept.plain_token,
    invitation_expired_id: invitation_expired.id,
    invitation_expired_email: invitation_expired_account.email,
    invitation_expired_account_id: invitation_expired_account.id,
    invitation_expired_access_token: invitation_expired_access_token,
    invitation_expired_token: invitation_expired.plain_token,
    invitation_revoked_id: invitation_revoked.id,
    invitation_revoked_access_token: invitation_revoked_access_token,
    invitation_revoked_token: invitation_revoked.plain_token,
    invitation_rotation_id: invitation_rotation.id,
    invitation_rotation_email: invitation_rotation_account.email,
    invitation_rotation_access_token: invitation_rotation_access_token,
    invitation_rotation_token: invitation_rotation.plain_token,
    invitation_mobile_token: invitation_mobile.plain_token,
    invitation_mobile_oauth_token: invitation_mobile_oauth_token,
    invitation_duplicate_email: invitation_duplicate_email,
    admin_target_membership_id: admin_target_membership.id,
    admin_target_access_token: admin_target_access_token,
    admin_owner_patch_membership_id: admin_owner_patch_membership.id,
    admin_manager_put_membership_id: admin_manager_put_membership.id,
    admin_manager_put_access_token: admin_manager_put_access_token,
    admin_owner_put_membership_id: admin_owner_put_membership.id,
    admin_owner_put_access_token: admin_owner_put_access_token,
    admin_invalid_membership_id: admin_invalid_membership.id,
    admin_revoke_membership_id: admin_revoke_membership.id,
    admin_revoke_access_token: admin_revoke_access_token,
    last_owner_household_id: last_owner_household.id,
    last_owner_membership_id: last_owner_membership.id,
    last_owner_access_token: last_owner_access_token,
    session_id: session.id,
    revocable_session_id: revocable_session.id,
    revocable_access_token: revocable_access_token,
    logout_access_token: logout_access_token,
    expired_access_token: expired_access_token,
    locked_access_token: locked_access_token,
    auth_deactivated_household_id: deactivated_household.id,
    auth_deactivated_access_token: deactivated_access_token,
    auth_deactivated_app_token: deactivated_app_token,
    auth_inactive_household_id: inactive_household.id,
    auth_inactive_access_token: inactive_access_token,
    auth_operational_states: operational_states,
    auth_suspended_household_id: suspended_household.id,
    auth_suspended_access_token: suspended_access_token,
    auth_role_household_id: role_household.id,
    auth_role_member_membership_id: role_member_membership.id,
    auth_role_owner_access_token: role_owner_access_token,
    auth_role_member_access_token: role_member_access_token,
    auth_role_member_app_token: role_member_app_token,
    auth_role_member_oauth_token: role_member_oauth_token,
    oauth_client_id: oauth_client_id,
    oauth_redirect_uri: oauth_redirect_uri,
    user_person_id: account.person.id,
    managed_person_id: managed_person.id,
    managed_person_portable_id: managed_person.portable_id,
    hidden_person_id: hidden_person.id,
    hidden_person_portable_id: hidden_person.portable_id,
    foreign_person_id: foreign_account.person.id,
    foreign_person_portable_id: foreign_account.person.portable_id,
    foreign_person_name: foreign_account.person.name,
    view_access_token: view_access_token,
    feed_access_token: feed_access_token,
    cursor_boundary_location_portable_id: cursor_boundary_location_portable_id,
    view_account_id: view_account.id,
    view_membership_id: view_membership.id,
    delegated_access_token: delegated_access_token,
    replay_access_token: replay_access_token,
    replay_mobile_access_token: replay_mobile_access_token,
    replay_grant_id: replay_grant.id,
    view_owner_access_token: view_owner_access_token,
    care_access_token: care_access_token,
    grant_target_membership_id: grant_target_membership.id,
    primary_location_id: primary_location.id,
    primary_location_portable_id: primary_location.portable_id,
    hidden_location_portable_id: hidden_location.portable_id,
    historical_location_portable_id: historical_location.portable_id,
    historical_medication_portable_id: historical_medication.portable_id,
    historical_medication_id: historical_medication.id,
    historical_medication_name: historical_medication.name,
    foreign_location_id: foreign_location.id,
    foreign_location_portable_id: foreign_location.portable_id,
    foreign_location_name: foreign_location.name,
    managed_medication_id: managed_medication.id,
    managed_medication_portable_id: managed_medication.portable_id,
    managed_medication_name: managed_medication.name,
    visible_low_stock_portable_id: visible_low_stock.portable_id,
    hidden_low_stock_portable_id: hidden_low_stock.portable_id,
    foreign_low_stock_portable_id: foreign_low_stock.portable_id,
    managed_dosage_portable_id: managed_dosage.portable_id,
    hidden_medication_id: hidden_medication.id,
    hidden_medication_portable_id: hidden_medication.portable_id,
    foreign_medication_id: foreign_medication.id,
    foreign_medication_portable_id: foreign_medication.portable_id,
    foreign_medication_name: foreign_medication.name,
    managed_assignment_id: managed_assignment.id,
    managed_assignment_portable_id: managed_assignment.portable_id,
    fhir_stopped_assignment_portable_id: fhir_stopped_assignment.portable_id,
    managed_assignment_updated_at: managed_assignment_updated_at.iso8601,
    retired_assignment_portable_id: retired_assignment.portable_id,
    retired_assignment_period_id: retired_assignment_period.portable_id,
    hidden_assignment_id: hidden_assignment.id,
    hidden_assignment_portable_id: hidden_assignment.portable_id,
    foreign_assignment_id: foreign_assignment.id,
    foreign_assignment_portable_id: foreign_assignment.portable_id,
    hidden_pause_period_id: hidden_pause_period.portable_id,
    foreign_pause_period_id: foreign_pause_period.portable_id,
    managed_schedule_id: managed_schedule.id,
    managed_schedule_portable_id: managed_schedule.portable_id,
    fhir_stopped_schedule_portable_id: fhir_stopped_schedule.portable_id,
    historical_schedule_portable_id: historical_schedule.portable_id,
    managed_occurrence_portable_id: managed_occurrence.portable_id,
    managed_take_portable_id: managed_take.portable_id,
    hidden_take_portable_id: hidden_take.portable_id,
    foreign_take_portable_id: foreign_take.portable_id,
    managed_preference_portable_id: managed_preference.portable_id,
    hidden_preference_portable_id: hidden_preference.portable_id,
    foreign_preference_portable_id: foreign_preference.portable_id,
    managed_health_event_portable_id: managed_health_event.portable_id,
    managed_health_event_id: managed_health_event.id,
    managed_health_event_title: managed_health_event.title,
    managed_side_effect_title: managed_side_effect.title,
    earlier_health_event_id: earlier_health_event.id,
    hidden_health_event_portable_id: hidden_health_event.portable_id,
    foreign_health_event_portable_id: foreign_health_event.portable_id,
    hidden_schedule_id: hidden_schedule.id,
    hidden_schedule_portable_id: hidden_schedule.portable_id,
    foreign_schedule_id: foreign_schedule.id,
    foreign_schedule_portable_id: foreign_schedule.portable_id,
    foreign_dosage_id: foreign_dosage.id,
    foreign_dosage_portable_id: foreign_dosage.portable_id,
    hidden_dosage_id: hidden_dosage.id,
    hidden_dosage_portable_id: hidden_dosage.portable_id,
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
