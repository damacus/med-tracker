require 'rails_helper'

RSpec.describe 'Web dose outcomes' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:source) { schedules(:john_movicol).reload }
  let(:key) do
    MedicationAdministration::OccurrenceProjection.new(source: source, start_date: Date.current,
                                                       end_date: Date.current).call.first.key
  end

  before do
    sign_in(users(:admin))
    source.update!(frequency: 'Daily', schedule_type: :daily, schedule_config: {},
                   start_date: Date.yesterday, end_date: Date.tomorrow, max_daily_doses: 1)
  end

  it 'shows an accessible confirmation with the person, medicine and optional context' do
    get new_schedule_dose_occurrence_path(source)
    expect(response).to have_http_status(:ok)
    expect(response.body).to include(source.person.name, source.medication.display_name, 'Confirm not taken')
    document = response.parsed_body
    expect(document.at_css('label[for="outcome_reason"]')).to be_present
    expect(document.at_css('label[for="outcome_note"]')).to be_present
    expect(document.at_css('form[data-turbo-frame="_top"]')).to be_present
  end

  it 'records a decision and returns to the dashboard without a take or stock change' do
    supply = source.medication.current_supply
    expect do
      post schedule_dose_occurrences_path(source),
           params: { dose_occurrence: { key: key, reason: 'unwell', note: 'Resting' } }
    end.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:see_other)
    expect(response).to redirect_to(dashboard_path(dashboard_person_id: source.person_id))
    expect(source.medication_dose_occurrences.sole).to have_attributes(outcome: 'not_taken', note: 'Resting')
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'renders a validation error with the submitted note preserved' do
    post schedule_dose_occurrences_path(source),
         params: { dose_occurrence: { key: key, reason: 'invalid', note: 'Resting' } }
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('role="alert"', 'Resting')
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'does not show a confirm button when every dose has been resolved' do
    actor = users(:admin).person.account.first_active_household_membership
    source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1, outcome: 'not_taken',
                                               resolved_at: Time.current,
                                               resolved_by_membership: actor)
    get new_schedule_dose_occurrence_path(source)
    expect(response).to have_http_status(:ok)
    expect(response.body).to include('No doses are waiting for a decision')
    expect(response.body).not_to include('Confirm not taken')
  end

  it 'rejects a forged occurrence key without disclosing it' do
    post schedule_dose_occurrences_path(source), params: { dose_occurrence: { key: 'private-forged-key' } }
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private-forged-key')
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'offers a labelled dashboard action for a due formal schedule' do
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    document = response.parsed_body
    link = document.at_css("a[href='#{new_schedule_dose_occurrence_path(source)}']")
    expect(link).to be_present
    expect(link.text).to include('Not taken')
  end

  it 'rejects future doses even when a valid key is submitted' do
    travel_to Time.current.change(hour: 12) do
      source.update!(schedule_config: { 'times' => ['20:00'] })
      post schedule_dose_occurrences_path(source), params: { dose_occurrence: { key: key } }
      expect(response).to have_http_status(:unprocessable_content)
      expect(source.medication_dose_occurrences).to be_empty
    end
  end

  it 'rejects a viewer and hides the dashboard action' do
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id)
                     .find_each { |grant| grant.update!(access_level: :view) }
    post schedule_dose_occurrences_path(source), params: { dose_occurrence: { key: key } }
    expect(response).to redirect_to(root_path)
    expect(source.medication_dose_occurrences).to be_empty
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    expect(response.parsed_body.at_css("a[href='#{new_schedule_dose_occurrence_path(source)}']")).to be_nil
  end
end
