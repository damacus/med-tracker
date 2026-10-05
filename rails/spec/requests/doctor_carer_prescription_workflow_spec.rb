# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Doctor prescription with carer administration' do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :users, :carer_relationships,
           :dosages

  let(:doctor) { users(:doctor) }
  let(:carer) { users(:carer) }
  let(:patient) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }
  let(:dosage) { dosages(:paracetamol_child) }

  delegate :household, to: :patient

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
    PersonMedication.where(person: patient, medication: medication).delete_all
  end

  it 'allows a doctor to prescribe timing-limited medication and blocks an early carer dose' do
    sign_in(doctor)
    grant_patient_access(doctor, access_level: :manage, relationship_type: :professional)

    expect do
      post person_medication_assignments_path(patient, household_slug: household.slug),
           params: {
             medication_assignment: {
               medication_id: medication.id,
               source_dosage_option_id: dosage.id
             }
           }
    end.to change(PersonMedication, :count).by(1)

    prescription = PersonMedication.order(:id).last

    expect(response).to redirect_to(person_path(patient, household_slug: household.slug))
    expect(prescription).to have_attributes(
      person: patient,
      medication: medication,
      source_dosage_option: dosage,
      dose_amount: BigDecimal('250.0'),
      dose_unit: 'mg',
      max_daily_doses: 4,
      min_hours_between_doses: 4,
      administration_kind: 'as_needed'
    )
    expect(audit_event_for(prescription)).to be_present

    sign_in(carer)
    grant_patient_access(carer, access_level: :record, relationship_type: :carer)

    first_taken_at = Time.zone.local(2026, 5, 5, 9, 0)
    travel_to first_taken_at do
      expect do
        post take_medication_person_person_medication_path(patient, prescription, household_slug: household.slug),
             params: { medication_take: { taken_at: first_taken_at.strftime('%Y-%m-%dT%H:%M') } }
      end.to change(MedicationTake, :count).by(1)
    end

    take = MedicationTake.order(:id).last

    expect(response).to redirect_to(person_path(patient, household_slug: household.slug))
    expect(flash[:notice]).to include('Medication taken')
    expect(take).to have_attributes(
      person_medication: prescription,
      dose_amount: BigDecimal('250.0'),
      dose_unit: 'mg'
    )
    expect(audit_event_for(take)).to be_present

    travel_to first_taken_at + 1.hour do
      expect do
        post take_medication_person_person_medication_path(patient, prescription, household_slug: household.slug)
      end.not_to change(MedicationTake, :count)
    end

    expect(response).to redirect_to(person_path(patient, household_slug: household.slug))
    expect(flash[:alert]).to include('Cannot take medication')
  end

  def grant_patient_access(user, access_level:, relationship_type:)
    membership = household.household_memberships.find_by!(account: user.person.account)
    grantor = household.household_memberships.owner.active.first || membership
    grant = household.person_access_grants.find_or_initialize_by(household_membership: membership, person: patient)
    grant.update!(
      access_level: access_level,
      relationship_type: relationship_type,
      granted_by_membership: grantor
    )
  end

  def audit_event_for(record)
    PaperTrail::Version.where(item_type: record.class.name, item_id: record.id, event: 'create').first
  end
end
