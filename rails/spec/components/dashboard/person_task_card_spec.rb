require 'rails_helper'

RSpec.describe Components::Dashboard::PersonTaskCard, type: :component do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications

  it 'shows a not-taken outcome and its context without offering another take action' do
    source = schedules(:john_movicol)
    outcome = MedicationDoseOccurrence.new(schedule: source, outcome: 'not_taken', reason: 'unwell',
                                           note: 'Feeling unwell', scheduled_at: Time.current)
    component = described_class.new(person: source.person, routine_tasks: [], as_needed_items: [],
                                    not_taken_outcomes: [outcome])

    rendered = render_inline(component)

    expect(rendered.text).to include('Not taken', 'Unwell', 'Feeling unwell', source.medication.display_name)
    expect(rendered.css('[data-testid="dashboard-routine-task"]')).to be_empty
    expect(rendered.css('[data-testid="dashboard-not-taken-outcome"]').size).to eq(1)
  end

  it 'shows routine context without using a formal-schedule correction route' do
    source = person_medications(:john_vitamin_d)
    outcome = MedicationDoseOccurrence.new(id: 123, person_medication: source, outcome: 'not_taken',
                                           reason: 'unwell', note: 'Routine decision')
    component = described_class.new(person: source.person, routine_tasks: [], as_needed_items: [],
                                    not_taken_outcomes: [outcome], can_correct_outcomes: true)
    rendered = render_inline(component)
    expect(rendered.text).to include('Routine decision', source.medication.display_name)
    expect(rendered.css('a[href*="/schedules/"]')).to be_empty
  end
end
