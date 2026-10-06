require 'rails_helper'

RSpec.describe 'Web routine dose outcomes' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                         dose_cycle: :monthly, max_daily_doses: 1, created_at: 2.months.ago)
  end

  before { sign_in(users(:admin)) }

  def path = person_medication_dose_occurrences_path(source)

  def key
    MedicationAdministration::OccurrenceProjection.new(source: source, start_date: Date.current,
                                                       end_date: Date.current).call.first.key
  end

  it 'shows an accessible confirmation with the complete routine cycle window' do
    get "#{path}/new"
    expect(response).to have_http_status(:ok)
    expect(response.body).to include(source.person.name, source.medication.display_name, 'Confirm not taken',
                                     Date.current.end_of_month.iso8601)
    expect(response.parsed_body.at_css('label[for="outcome_reason"]')).to be_present
    expect(response.parsed_body.at_css('form[data-turbo-frame="_top"]')[:action]).to eq(path)
  end

  it 'records a routine decision and returns to the dashboard without changing stock' do
    supply = source.medication.current_supply
    expect do
      post path, params: { dose_occurrence: { key: key, reason: 'unwell', note: 'Routine decision' } }
    end.not_to change(MedicationTake, :count)
    expect(response).to redirect_to(dashboard_path(dashboard_person_id: source.person_id))
    expect(response).to have_http_status(:see_other)
    expect(source.medication_dose_occurrences.sole).to be_not_taken
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'preserves submitted context when validation fails' do
    post path, params: { dose_occurrence: { key: key, reason: 'invalid', note: 'Routine decision' } }
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('role="alert"', 'Routine decision')
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'offers a labelled action on the routine dashboard row' do
    source
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    link = response.parsed_body.at_css("a[href='#{path}/new']")
    expect(link).to be_present
    expect(link.text).to eq('Not taken')
  end

  it 'omits as-needed choices and rejects a former routine key' do
    identity = key
    source.update!(administration_kind: :as_needed)
    get "#{path}/new"
    expect(response).to have_http_status(:ok)
    expect(response.body).not_to include('Confirm not taken')
    post path, params: { dose_occurrence: { key: identity } }
    expect(response).to have_http_status(:unprocessable_content)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'rejects a viewer and hides the dashboard action' do
    actor = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: actor, person_id: source.person_id)
                     .find_each { |grant| grant.update!(access_level: :view) }
    post path, params: { dose_occurrence: { key: key } }
    expect(response).to redirect_to(root_path)
    expect(source.medication_dose_occurrences).to be_empty
    get dashboard_path, params: { dashboard_person_id: source.person_id }
    expect(response.parsed_body.at_css("a[href='#{path}/new']")).to be_nil
  end
end
