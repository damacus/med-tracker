# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::HealthHistoryPdf do
  let(:start_date) { Date.new(2026, 2, 1) }
  let(:end_date) { Date.new(2026, 2, 28) }
  let(:generated_at) { Time.utc(2026, 3, 1, 10, 30) }

  it 'renders the empty health-history report through the shared PDF renderer' do
    pdf = described_class.new(result: empty_result, start_date:, end_date:, generated_at:).render

    expect(pdf_metadata(pdf)).to eq(
      'Title' => 'MedTracker health history report',
      'Author' => 'MedTracker'
    )
    expect(pdf_text(pdf)).to include(
      'People:',
      'No records in this section.',
      'Disclaimer',
      'Generated: 2026-03-01 10:30 UTC'
    )
  end

  it 'retains long notes, report context, metadata and all locale glyphs' do
    long_note_prefix = 'Long note survives PDF rendering.'
    long_note_tail = 'Final health-history note marker: Cymraeg ŵ · Español ñ · Gaeilge á · Português ç.'
    long_note = "#{"#{long_note_prefix} " * 12}#{long_note_tail}"
    result = populated_result(notes: long_note)
    pdf = described_class.new(result:, start_date:, end_date:, generated_at:).render

    expect(pdf_metadata(pdf)).to eq(expected_pdf_metadata)
    expect(pdf_text(pdf)).to include(
      'People:', 'Date range:', 'Alex Smith', long_note_prefix, long_note_tail.unicode_normalize(:nfkd)
    )
    expect(Components::Reports::HealthHistoryReport.new(result:).call).to include(long_note)
    expect(unicode_map(pdf_streams(pdf))).to include('<0136> <0175>', '<00B3> <00F1>', '<00A3> <00E1>', '<00A9> <00E7>')
  end

  it 'does not pass patient context to the PDF metadata' do
    renderer = instance_spy(Reports::PdfRenderer)
    allow(Reports::PdfRenderer).to receive(:new).and_return(renderer)

    described_class.new(result: populated_result, start_date:, end_date:, generated_at:).render

    expect(renderer).to have_received(:render).with(
      component: an_instance_of(Components::Reports::PdfDocument),
      metadata: report_metadata
    )
  end

  it 'renders translated Welsh report copy' do
    pdf = I18n.with_locale(:cy) do
      described_class.new(result: empty_result, start_date:, end_date:, generated_at:).render
    end

    expect(pdf_text(pdf)).to include('Pobl:', 'Ymwadiad')
    expect(pdf_text(pdf)).not_to include('People:')
  end

  it 'renders a large health-history table across multiple pages' do
    medication_takes = Array.new(240) { |index| medication_take(index:) }
    pdf = described_class.new(
      result: empty_result.with(medication_takes:),
      start_date:,
      end_date:,
      generated_at:
    ).render

    expect(pdf.scan(%r{/Type /Page\b}).size).to be > 1
    expect(pdf_text(pdf)).to include('Paracetamol')
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

  def report_metadata
    { title: 'MedTracker health history report', author: 'MedTracker' }
  end

  def expected_pdf_metadata
    { 'Title' => 'MedTracker health history report', 'Author' => 'MedTracker' }
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

  def unicode_map(streams)
    streams.find { it.include?('beginbfchar') }
  end
end
