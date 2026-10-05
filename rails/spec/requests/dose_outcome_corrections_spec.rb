require 'rails_helper'

RSpec.describe 'Web dose outcome corrections' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:source) { schedules(:john_movicol).reload }
  let(:actor) { users(:admin).person.account.first_active_household_membership }
  let(:outcome) do
    source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1, outcome: 'not_taken',
                                               reason: 'unwell', note: 'Resting', resolved_at: Time.current,
                                               resolved_by_membership: actor)
  end

  before do
    sign_in(users(:admin))
    outcome
  end

  def correct(resolution, **attributes)
    patch schedule_dose_occurrence_path(source, outcome),
          params: { dose_occurrence: { resolution: resolution, etag: Api::RecordEtag.for(outcome) }.merge(attributes) }
  end

  it 'shows the observed decision and version before correction' do
    get edit_schedule_dose_occurrence_path(source, outcome)
    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Resting', 'Reopen dose', 'Record as taken')
    expect(response.parsed_body.at_css('input[name="dose_occurrence[etag]"]')['value'])
      .to eq(Api::RecordEtag.for(outcome))
  end

  it 'links the dashboard decision to its correction form' do
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    link = response.parsed_body.at_css("a[href='#{edit_schedule_dose_occurrence_path(source, outcome)}']")
    expect(link).to be_present
    expect(link.text).to include('Correct decision')
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
    get edit_schedule_dose_occurrence_path(source, outcome)
    expect(response.body).to include('Record as taken')
    expect(response.body).not_to include('Reopen dose')
    correct('reopen')
    expect(response).to redirect_to(root_path)
    expect(outcome.reload).to be_not_taken
  end

  it 'rejects an administration time from another day' do
    correct('take', taken_at: "#{Date.yesterday.iso8601}T12:00")
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
end
