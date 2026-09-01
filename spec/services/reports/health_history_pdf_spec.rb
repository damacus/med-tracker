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

  it 'renders the single-person GP report through the shared PDF renderer' do
    pdf = described_class.new(result: gp_result, start_date:, end_date:, generated_at:).render

    expect(pdf_metadata(pdf)).to eq(expected_pdf_metadata)
    expect(pdf_text(pdf)).to include(*gp_pdf_content, 'Alex Smith', 'Reporting period: 2026-02-01 to 2026-02-28',
                                     'Generated: 2026-03-01 10:30 UTC')
    expect(Components::Reports::GpHealthHistoryReport.new(result: gp_result).call)
      .to include('Date of birth: 1990-01-01')
  end

  it 'keeps medication administrations out of the GP report unless the appendix is requested' do
    without_appendix = described_class.new(result: gp_result, start_date:, end_date:, generated_at:).render
    with_appendix = described_class.new(
      result: gp_result,
      start_date:,
      end_date:,
      generated_at:,
      include_medication_takes: true
    ).render

    expect(pdf_text(without_appendix)).not_to include('Medication administrations appendix', '2026-02-15 08:30')
    expect(pdf_text(with_appendix)).to include('Medication administrations appendix', '2026-02-15 08:30', 'Paracetamol')
    expect(Components::Reports::GpHealthHistoryReport.new(result: gp_result, include_medication_takes: true).call)
      .to include('pdf-appendix', 'health-history-medication-table')
  end

  it 'puts the medication administration appendix on the page after the main GP report' do
    pdf = described_class.new(
      result: gp_result.with(medication_takes: Array.new(90) { |index| medication_take(index:) }),
      start_date:,
      end_date:,
      generated_at:,
      include_medication_takes: true
    ).render
    page_texts = pdf_page_texts(pdf)
    disclaimer_page = page_texts.index { it.include?('Disclaimer') }
    appendix_page = page_texts.index { it.include?('Medication administrations appendix') }

    expect(disclaimer_page).to be < appendix_page
    expect(appendix_page).to eq(disclaimer_page + 1)
  end

  it 'renders every long chronology sentinel with repeated table headings' do
    result = gp_result.with(chronology: long_chronology)
    pdf = described_class.new(result:, start_date:, end_date:, generated_at:).render

    expect(pdf.scan(%r{/Type /Page\b}).size).to be > 1
    expect(pdf_text(pdf)).to include('Recorded episode 1', 'Long chronology sentinel 50')
    expect(Components::Reports::GpHealthHistoryReport.new(result:).call)
      .to include('health-history-chronology-table', '<thead>', 'Dates', 'Type', 'Details')

    chronology_pages = pdf_page_texts(pdf).select { it.include?('Recorded episode') }
    expect(chronology_pages.drop(1)).to all(include('DATS TP TTL DTALS'))
  end

  it 'localizes recorded chronology facts and omits optional facts that were not recorded' do
    pdf = I18n.with_locale(:es) { localized_optional_facts_pdf }

    expect(pdf_text(pdf)).to include(
      I18n.t('health_events.kinds.suspected_side_effect', locale: :es),
      I18n.t('health_events.severities.moderate', locale: :es)
    )
    expect(pdf_text(pdf)).not_to include(I18n.t('reports.health_history.gp.not_recorded', locale: :es))
  end

  it 'renders localized GP page numbering and linked-medicine connectors' do
    pdf = I18n.with_locale(:es) do
      described_class.new(
        result: gp_result(medication_names: %w[Paracetamol Ibuprofen]),
        start_date:,
        end_date:,
        generated_at:
      ).render
    end

    expect(pdf_text(pdf)).to include(
      'Medicamentos relacionados: Paracetamol y Ibuprofen',
      'Pgina de'
    )
  end

  it 'renders the complete long medication administration appendix with repeated headings' do
    result = gp_result.with(medication_takes: Array.new(90) { |index| medication_take(index:) })
    pdf = described_class.new(
      result:,
      start_date:,
      end_date:,
      generated_at:,
      include_medication_takes: true
    ).render

    expect(pdf.scan(%r{/Type /Page\b}).size).to be > 2
    expect(pdf_text(pdf)).to include('Medication administrations appendix', '2026-02-01 08:30', '2026-02-28 08:30')
    expect(Components::Reports::GpHealthHistoryReport.new(result:, include_medication_takes: true).call)
      .to include('health-history-medication-table', '<thead>', 'Time')

    appendix_start = pdf_page_texts(pdf).index { it.include?('Medication administrations appendix') }
    appendix_pages = pdf_page_texts(pdf)[appendix_start..]
    expect(appendix_pages).to all(include('TM PRSON MDCATON DOS SOURC LOCATON'))
  end

  def gp_pdf_content
    [
      'Person details',
      'Current medicines',
      'Paracetamol',
      'Chronology',
      'Nausea',
      'Severity: Moderate',
      'Started after evening dose',
      'Action taken: Called pharmacy',
      'Medical help sought: Yes',
      'Linked medicines: Paracetamol'
    ]
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

  def gp_result(medication_names: ['Paracetamol'])
    person = Data.define(:name, :date_of_birth).new('Alex Smith', Date.new(1990, 1, 1))
    medicine = Data.define(:display_name).new('Paracetamol')
    Data.define(:person, :current_medicines, :chronology, :medication_takes).new(
      person, [medicine], [health_event_entry(gp_event, medication_names)], [medication_take(index: 14, person:)]
    )
  end

  def gp_event
    health_event(
      title: 'Nausea',
      event_kind: :suspected_side_effect,
      started_on: Date.new(2026, 2, 10),
      ended_on: nil,
      severity: :moderate,
      notes: 'Started after evening dose',
      action_taken: 'Called pharmacy',
      medical_help_sought: true,
      ongoing?: true
    )
  end

  def long_chronology
    Array.new(50) do |index|
      long_chronology_entry(index)
    end
  end

  def long_chronology_entry(index)
    health_event_entry(health_event(
                         title: "Recorded episode #{index + 1}", event_kind: :illness,
                         started_on: start_date + (index % 20), ended_on: nil, severity: :moderate,
                         notes: "Long chronology sentinel #{index + 1}", action_taken: 'Recorded action',
                         medical_help_sought: false, ongoing?: true
                       ), [])
  end

  def localized_optional_facts_pdf
    result = gp_result.with(chronology: [health_event_entry(gp_event, ['Paracetamol']), optional_facts_entry])
    described_class.new(result:, start_date:, end_date:, generated_at:).render
  end

  def optional_facts_entry
    health_event_entry(health_event(
                         title: 'Cold', event_kind: :illness, started_on: start_date, ended_on: start_date + 2.days,
                         severity: nil, notes: nil, action_taken: nil, medical_help_sought: false, ongoing?: false
                       ), [])
  end

  def health_event_entry(event, medication_names)
    Reports::HealthHistoryQuery::HealthEventEntry.new(event, medication_names)
  end

  def sample_event(notes:)
    health_event(
      title: 'Nausea',
      event_kind: :suspected_side_effect,
      started_on: Date.new(2026, 2, 10),
      ended_on: nil,
      severity: :moderate,
      notes:,
      action_taken: 'Called pharmacy',
      medical_help_sought: false,
      ongoing?: true
    )
  end

  def sample_illness
    health_event(
      title: 'Cold',
      event_kind: :illness,
      started_on: Date.new(2026, 2, 12),
      ended_on: Date.new(2026, 2, 14),
      severity: :mild,
      notes: '',
      action_taken: '',
      medical_help_sought: false,
      ongoing?: false
    )
  end

  def health_event(**attributes)
    Data.define(:title, :event_kind, :started_on, :ended_on, :severity, :notes,
                :action_taken, :medical_help_sought, :ongoing?).new(
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
