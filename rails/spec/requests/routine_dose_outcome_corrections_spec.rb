require 'rails_helper'

RSpec.describe 'Web routine dose outcome corrections' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                         dose_cycle: :monthly, max_daily_doses: 1, created_at: 2.months.ago)
  end
  let(:actor) { users(:admin).person.account.first_active_household_membership }
  let(:outcome) do
    source.medication_dose_occurrences.create!(window_starts_on: Date.current.beginning_of_month, position: 1,
                                               outcome: 'not_taken',
                                               reason: 'unwell', note: 'Resting', resolved_at: Time.current,
                                               resolved_by_membership: actor)
  end

  before do
    travel_to Time.zone.local(2026, 9, 9, 12)
    sign_in(users(:admin))
    outcome
  end

  def correct(resolution, **attributes)
    patch person_medication_dose_occurrence_path(source, outcome),
          params: { dose_occurrence: { resolution: resolution, etag: Api::RecordEtag.for(outcome) }.merge(attributes) }
  end

  it 'shows the observed decision and version before correction' do
    get edit_person_medication_dose_occurrence_path(source, outcome)
    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Resting', 'Reopen dose', 'Record as taken', '2026-09-30')
    expect(response.parsed_body.at_css('input[name="dose_occurrence[etag]"]')['value'])
      .to eq(Api::RecordEtag.for(outcome))
  end

  it 'links the dashboard decision to its correction form' do
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    link = response.parsed_body.at_css("a[href='#{edit_person_medication_dose_occurrence_path(source, outcome)}']")
    expect(link).to be_present
    expect(link.text).to include('Correct decision')
  end

  it 'keeps a saved monthly decision discoverable after changing to a daily cycle' do
    source.update!(dose_cycle: :daily)

    get dashboard_path, params: { dashboard_person_id: source.person_id }

    link = response.parsed_body.at_css("a[href='#{edit_person_medication_dose_occurrence_path(source, outcome)}']")
    expect(link).to be_present
    expect(FamilyDashboard::NotTakenQuery.new(schedules: [],
                                              person_medications: [source]).for_source(source)).to be_empty
  end

  it 'corrects a later daily outcome without changing the overlapping monthly decision' do
    source.update!(dose_cycle: :daily)
    daily = source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1,
                                                       outcome: 'not_taken', reason: 'refused',
                                                       resolved_at: Time.current, resolved_by_membership: actor)

    patch person_medication_dose_occurrence_path(source, daily),
          params: { dose_occurrence: { resolution: 'reopen', etag: Api::RecordEtag.for(daily) } }

    expect(response).to have_http_status(:see_other)
    expect(daily.reload).to be_open
    expect(outcome.reload).to have_attributes(outcome: 'not_taken', reason: 'unwell')
  end

  it 'reopens without a take and retains the previous decision in the audit history' do
    expect { correct('reopen') }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:see_other)
    expect(outcome.reload).to be_open
    expect(outcome.versions.last.reify).to have_attributes(reason: 'unwell', note: 'Resting')
  end

  it 'rejects stale or missing versions without changing the decision' do
    [nil, 'stale'].each do |etag|
      correct('reopen', etag: etag)
      expect(response).to have_http_status(:unprocessable_content)
      expect(outcome.reload).to be_not_taken
    end
  end

  it 'records one linked take and stock deduction when replacing the decision' do
    source.medication.update!(current_supply: 100)
    attributes = { taken_at: Time.current.strftime('%Y-%m-%dT%H:%M'), client_uuid: SecureRandom.uuid,
                   taken_from_medication_id: source.medication_id }
    expect { correct('take', **attributes) }.to change(MedicationTake, :count).by(1)
    expect(response).to have_http_status(:see_other)
    expect(outcome.reload).to be_taken
    expect(outcome.versions.last.reify.note).to eq('Resting')
    supply = source.medication.reload.current_supply
    expect(supply).to be < 100
  end

  it 'replays an actual dose correction without a second take or stock change' do
    attributes = { taken_at: Time.current.strftime('%Y-%m-%dT%H:%M'), client_uuid: SecureRandom.uuid }
    correct('take', **attributes)
    expect(outcome.reload).to be_taken
    supply = source.medication.reload.current_supply
    expect { correct('take', **attributes) }.not_to change(MedicationTake, :count)
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'requires manage access to reopen but lets a recorder see replacement controls' do
    PersonAccessGrant.where(household_membership: actor, person_id: source.person_id)
                     .find_each { |grant| grant.update!(access_level: :record) }
    get edit_person_medication_dose_occurrence_path(source, outcome)
    expect(response.body).to include('Record as taken')
    expect(response.body).not_to include('Reopen dose')
    correct('reopen')
    expect(response).to redirect_to(root_path)
    expect(outcome.reload).to be_not_taken
  end

  it 'rejects an administration time from another cycle' do
    correct('take', taken_at: "#{Date.current.prev_month.iso8601}T12:00")
    expect(response).to have_http_status(:unprocessable_content)
    expect(outcome.reload).to be_not_taken
  end

  it 'does not reopen or edit a linked actual take' do
    correct('take', taken_at: Time.current.strftime('%Y-%m-%dT%H:%M'))
    expect(outcome.reload).to be_taken
    correct('reopen')
    expect(response).to have_http_status(:unprocessable_content)
    expect(outcome.reload).to be_taken
  end

  it 'keeps the saved cycle when the assignment frequency changes' do
    source.update!(dose_cycle: :daily)
    correct('take', taken_at: '2026-09-08T12:00')
    expect(response).to have_http_status(:see_other)
    expect(outcome.reload).to be_taken
    expect(outcome.window_ends_on).to eq(Date.new(2026, 9, 30))
  end

  it 'rejects a viewer before rendering or changing a saved decision' do
    PersonAccessGrant.where(household_membership: actor, person_id: source.person_id)
                     .find_each { |grant| grant.update!(access_level: :view) }
    get edit_person_medication_dose_occurrence_path(source, outcome)
    expect(response).to redirect_to(root_path)
    correct('take', taken_at: '2026-09-09T11:00')
    expect(response).to redirect_to(root_path)
    expect(outcome.reload).to be_not_taken
  end
end
