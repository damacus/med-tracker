# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Reports::HealthHistoryReport, type: :component do
  let(:start_date) { Date.new(2026, 2, 1) }
  let(:end_date) { Date.new(2026, 2, 28) }
  let(:generated_at) { Time.utc(2026, 3, 1, 10, 30) }

  it 'renders an empty state for every report section and retains the translated disclaimer' do
    rendered = Nokogiri::HTML5(described_class.new(result: empty_result).call)

    expect(rendered.css('.empty-state').map(&:text)).to all(include('No records in this section.'))
    expect(rendered.css('.empty-state').size).to eq(4)
    expect(rendered.css('h2').map(&:text)).to eq(expected_section_titles)
    expect(rendered.text).to include('This report reflects information entered into MedTracker.')
  end

  it 'uses every supported locale for report section content' do
    %i[en cy es ga pt].each do |locale|
      rendered = I18n.with_locale(locale) do
        Nokogiri::HTML5(described_class.new(result: empty_result).call)
      end

      expect(rendered.css('h2').first.text).to eq(I18n.t('reports.health_history.medication_takes.title', locale:))
      expect(rendered.text).to include(I18n.t('reports.health_history.disclaimer.medical_advice', locale:))
    end
  end

  it 'renders health-history rows with long names and notes without querying records' do
    long_name = 'Avery ' * 40
    long_note = 'Detailed note about symptoms and care. ' * 80
    report = described_class.new(result: populated_result(person_name: long_name, notes: long_note))
    rendered = Nokogiri::HTML5(report.call)

    expect(rendered.css('table').size).to eq(3)
    expect(rendered.text).to include(long_name, long_note, 'Paracetamol', 'Called pharmacy')
    expect(rendered.text).to include('2 "Cold" episodes were recorded')
  end

  it 'renders multilingual glyphs in the report content' do
    rendered = Nokogiri::HTML5(
      described_class.new(result: populated_result(notes: 'Cymraeg ŵ · Español ñ · Gaeilge á · Português ç')).call
    )

    expect(rendered.text).to include('Cymraeg ŵ · Español ñ · Gaeilge á · Português ç')
  end

  it 'renders a large table with a header and every medication row' do
    medication_takes = Array.new(240) do |index|
      medication_take(index:)
    end
    result = empty_result.with(medication_takes:)
    rendered = Nokogiri::HTML5(described_class.new(result:).call)

    headings = rendered.css('table').first.css('thead th').map(&:text)

    expect(headings).to eq(%w[Time Person Medication Dose Source Location])
    expect(rendered.css('table').first.css('tbody tr').size).to eq(240)
    populated = Nokogiri::HTML5(described_class.new(result: populated_result).call)

    expect(populated.css('table').map { |table| table['class'] }).to include(
      'health-history-medication-table',
      'health-history-side-effects-table',
      'health-history-illnesses-table'
    )
  end

  def empty_result
    Reports::HealthHistoryQuery::Result.new(
      people: [],
      medication_takes: [],
      suspected_side_effects: [],
      notable_illnesses: [],
      illness_patterns: []
    )
  end

  def populated_result(person_name: 'Alex Smith', notes: 'Started after evening dose')
    person = Data.define(:name).new(person_name)
    empty_result.with(
      people: [person],
      medication_takes: [medication_take(person:)],
      suspected_side_effects: [health_event_entry(sample_event(notes:), ['Paracetamol'])],
      notable_illnesses: [health_event_entry(sample_illness, [])],
      illness_patterns: [sample_pattern]
    )
  end

  def health_event_entry(event, medication_names)
    Reports::HealthHistoryQuery::HealthEventEntry.new(event, medication_names)
  end

  def sample_event(notes:)
    health_event(
      title: 'Nausea',
      started_on: Date.new(2026, 2, 10),
      ended_on: nil,
      severity: :moderate,
      notes:,
      action_taken: 'Called pharmacy',
      ongoing?: true
    )
  end

  def sample_illness
    health_event(
      title: 'Cold',
      started_on: Date.new(2026, 2, 12),
      ended_on: Date.new(2026, 2, 14),
      severity: :mild,
      notes: '',
      action_taken: '',
      ongoing?: false
    )
  end

  def health_event(**attributes)
    Data.define(:title, :started_on, :ended_on, :severity, :notes, :action_taken, :ongoing?).new(
      **attributes
    )
  end

  def sample_pattern
    Data.define(
      :episode_count,
      :display_title,
      :first_started_on,
      :most_recent_started_on,
      :average_interval_days
    ).new(2, 'Cold', Date.new(2026, 2, 1), Date.new(2026, 2, 20), 19)
  end

  def expected_section_titles
    [
      'Medication administrations',
      'Suspected side effects',
      'Notable illnesses',
      'Recorded illness patterns',
      'Disclaimer'
    ]
  end

  def medication_take(index: 0, person: Data.define(:name).new('Alex Smith'))
    Reports::HealthHistoryQuery::MedicationTakeEntry.new(
      person,
      Time.zone.parse("2026-02-#{format('%02d', (index % 28) + 1)} 08:30"),
      'Paracetamol',
      500,
      'mg',
      :scheduled,
      'Kitchen'
    )
  end
end
