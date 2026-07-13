# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Health events' do
  fixtures :accounts, :people, :users, :locations, :medications, :schedules, :person_medications

  let(:person) { people(:john) }

  before { sign_in(users(:admin)) }

  it 'lists person-scoped health events chronologically' do
    HealthEvent.create!(person: person, event_kind: :illness, title: 'Cold', started_on: Date.new(2026, 2, 1))

    get person_health_events_path(person)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Cold')
    expect(response.body).to include('Record notable illness')
    expect(response.body).to include('Record suspected side effect')
  end

  it 'renders the new form with valid and invalid event kind params' do
    get new_person_health_event_path(person), params: { event_kind: 'suspected_side_effect' }

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Record suspected side effect')

    get new_person_health_event_path(person), params: { event_kind: 'unknown' }

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Record notable illness')
  end

  it 'renders the edit form with selected medication links' do
    medication = medications(:paracetamol)
    event = HealthEvent.create!(person: person, event_kind: :suspected_side_effect, title: 'Nausea',
                                started_on: Date.new(2026, 2, 10))
    HealthEventMedication.create!(health_event: event, medication: medication)

    get edit_person_health_event_path(person, event)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Nausea')
    expect(response.body).to include('Paracetamol')
  end

  it 'creates a notable illness' do
    expect do
      post person_health_events_path(person),
           params: { health_event: illness_params(title: 'Tonsillitis', started_on: '2026-02-01') }
    end.to change(HealthEvent.illness, :count).by(1)

    expect(response).to redirect_to(person_health_events_path(person))
  end

  it 'creates a suspected side effect with authorised medication snapshots' do
    medication = medications(:paracetamol)

    expect do
      post person_health_events_path(person),
           params: {
             health_event: side_effect_params(title: 'Nausea', started_on: '2026-02-10'),
             medication_ids: [medication.id]
           }
    end.to(
      change(HealthEvent.suspected_side_effect, :count).by(1)
        .and(change { ApiChangeEvent.where(record_type: 'HealthEvent', action: 'update').count }.by(1))
    )

    event = HealthEvent.suspected_side_effect.order(:id).last
    expect(event.health_event_medications.sole.medication_name).to eq('Paracetamol')
  end

  it 're-renders the form when creation fails validation' do
    expect do
      post person_health_events_path(person),
           params: { health_event: illness_params(title: '', started_on: '') }
    end.not_to change(HealthEvent, :count)

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('Capture facts entered by the family or carer.')
  end

  it 'rejects medication links that are not assigned to the selected person' do
    unassigned_medication = create(:medication, name: 'Unassigned stock')

    post person_health_events_path(person),
         params: {
           health_event: side_effect_params(title: 'Nausea', started_on: '2026-02-10'),
           medication_ids: [unassigned_medication.id]
         }

    expect(response).to have_http_status(:unprocessable_content)
    expect(HealthEvent.suspected_side_effect.where(title: 'Nausea')).to be_empty
  end

  it 'updates an event and clears the end date when ongoing is selected' do
    event = HealthEvent.create!(
      person: person,
      event_kind: :illness,
      title: 'Cold',
      started_on: Date.new(2026, 2, 1),
      ended_on: Date.new(2026, 2, 4)
    )

    patch person_health_event_path(person, event),
          params: { health_event: illness_params(title: 'Cold again', started_on: '2026-02-01', ongoing: '1') }

    expect(response).to redirect_to(person_health_events_path(person))
    expect(event.reload).to have_attributes(title: 'Cold again', ended_on: nil)
  end

  it 're-renders the form when updates fail validation' do
    event = HealthEvent.create!(person: person, event_kind: :illness, title: 'Cold', started_on: Date.new(2026, 2, 1))

    patch person_health_event_path(person, event),
          params: { health_event: illness_params(started_on: '2026-02-10', ended_on: '2026-02-01') }

    expect(response).to have_http_status(:unprocessable_content)
    expect(event.reload.started_on).to eq(Date.new(2026, 2, 1))
  end

  it 'deletes an event' do
    event = HealthEvent.create!(person: person, event_kind: :illness, title: 'Cold', started_on: Date.new(2026, 2, 1))

    expect do
      delete person_health_event_path(person, event)
    end.to change(HealthEvent, :count).by(-1)
  end

  it 'does not expose unrelated people to another user' do
    post '/logout'
    sign_in(users(:parent))

    get person_health_events_path(person)

    expect(response).to have_http_status(:not_found)
  end

  def illness_params(**overrides)
    {
      event_kind: 'illness',
      title: 'Cold',
      started_on: '2026-02-01',
      ended_on: '',
      severity: 'mild',
      notes: 'Fever and sore throat',
      action_taken: '',
      medical_help_sought: '0'
    }.merge(overrides)
  end

  def side_effect_params(**overrides)
    illness_params(event_kind: 'suspected_side_effect', severity: 'moderate', notes: 'Started after dose')
      .merge(overrides)
  end
end
