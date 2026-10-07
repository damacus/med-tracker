rails_root = ENV.fetch('MEDTRACKER_REHEARSAL_RAILS_ROOT')
require File.join(rails_root, 'spec/spec_helper')
require 'uri'
raise 'Rails rollback rehearsal requires the test environment' unless ENV['RAILS_ENV'] == 'test'
raise 'Rails rollback rehearsal requires an owned database' unless ENV['MEDTRACKER_OWNED_DATABASE'] == '1'
raise 'Rails rollback rehearsal requires the saved-state target' unless URI(ENV.fetch('DATABASE_URL')).path == '/medtracker_reference'
require File.join(rails_root, 'config/environment')

RSpec.describe 'Rails saved-state rollback' do
  it 'signs in and records a dose on the restored synthetic household' do
    account = Account.find(71001)
    household = Household.find(72001)
    person = Person.find(73001)
    medication = Medication.find(80001)
    person_medication = PersonMedication.find(81001)

    expect(account.email).to eq('persistence@example.test')
    expect(household.slug).to eq('persistence-fixture')
    expect(person.household_id).to eq(household.id)
    expect(person_medication.medication_id).to eq(medication.id)
    expect(medication.current_supply).to eq(10)
    expect(PaperTrail::Version.where(item_type: 'Account', item_id: account.id,
                                     event: 'cutover_fixture/preserved').count).to eq(1)

    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.post('/login', params: { email: account.email, password: 'password' })
    expect(session.response).to be_redirect, "HTTP #{session.response.status}: #{session.response.body.first(4000)}"

    session.get("/households/#{household.slug}/dashboard")
    expect(session.response.status).to eq(200)

    before = MedicationTake.where(taken_from_medication_id: medication.id).count
    dose_path = Rails.application.routes.url_helpers.take_medication_person_person_medication_path(
      person, person_medication, household_slug: household.slug
    )
    session.post(dose_path,
                 params: { taken_from_medication_id: medication.id })
    expect(session.response).to be_redirect, "HTTP #{session.response.status}: #{session.response.body.first(4000)}"
    expect(MedicationTake.where(taken_from_medication_id: medication.id).count).to eq(before + 1)
    expect(medication.reload.current_supply).to be < 10
  end
end
