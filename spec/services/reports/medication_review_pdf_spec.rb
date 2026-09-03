require 'rails_helper'

RSpec.describe Reports::MedicationReviewPdf do
  let(:generated_at) { Time.utc(2026, 3, 1, 10, 30) }
  let(:boundary_text) do
    'This record organises public medicine-label evidence for discussion with a practitioner. ' \
      'It does not replace clinical judgement or tell someone to change a medicine.'
  end
  let(:pdf_content) do
    [
      'MedTracker medicine review record',
      'Prepared 1 March 2026 at 10:30 UTC',
      'Alex Smith',
      'High source risk',
      'The public label describes a serious risk.',
      'The medicines match a reviewed public-label evidence pair.',
      'Matched term: ibuprofen',
      'curated match',
      'Source instruction category: Label says avoid',
      'DailyMed',
      'Label version: 4',
      '1 July 2026',
      'https://example.test/evidence',
      'This record organises public medicine-label evidence for discussion with a practitioner.',
      'Reviewed with Dr Taylor',
      'GP on 9 July 2026.',
      'Recorded outcome: expected as prescribed.'
    ]
  end
  let(:prompt_attributes) do
    {
      person: Data.define(:id, :name).new(1, 'Alex Smith'),
      primary_medication_name: 'Warfarin',
      interacting_medication_name: 'Ibuprofen',
      risk_level: 'high',
      match_confidence: 'high',
      status: 'expected_prescribed_combination',
      evidence_text: 'The public label describes a serious risk.',
      match_reason: 'The medicines match a reviewed public-label evidence pair.',
      matched_term: 'ibuprofen',
      match_type: 'curated',
      source_instruction: 'avoid',
      evidence_source_name: 'DailyMed',
      evidence_source_version: '4',
      evidence_source_effective_on: Date.new(2026, 7, 1),
      evidence_source_checked_on: Date.new(2026, 7, 9),
      evidence_source_url: 'https://example.test/evidence',
      practitioner_name: 'Dr Taylor',
      practitioner_role: 'GP',
      reviewed_on: Date.new(2026, 7, 9),
      review_note: 'Confirmed as prescribed.',
      practitioner_review_status?: true
    }
  end

  it 'renders medication-review content and metadata through the shared renderer' do
    pdf = described_class.new(prompts: [prompt], generated_at:).render
    expect(pdf_metadata(pdf)).to eq(
      'Title' => 'MedTracker medicine review record',
      'Author' => 'MedTracker',
      'Subject' => boundary_text
    )
    aggregate_failures do
      pdf_content.each { |text| expect(pdf_text(pdf)).to include(text), "missing parsed PDF text: #{text}" }
    end
  end

  it 'does not pass patient context to the PDF metadata' do
    renderer = instance_spy(Reports::PdfRenderer)
    allow(Reports::PdfRenderer).to receive(:new).and_return(renderer)

    described_class.new(prompts: [prompt], generated_at:).render

    expect(renderer).to have_received(:render).with(
      component: an_instance_of(Components::Reports::PdfDocument),
      metadata: {
        title: 'MedTracker medicine review record',
        author: 'MedTracker',
        subject: boundary_text
      }
    )
  end

  it 'repeats person identity without losing prompts on continuation pages' do
    prompts = 8.times.map { |index| prompt(index:) }
    pdf = described_class.new(prompts:, generated_at:).render
    pages = pdf_page_texts(pdf)
    prompt_pages = pages.select { |page| page.include?('Evidence marker') }

    expect(pages.size).to be > 1
    expect(prompt_pages.size).to be > 1
    expect(prompt_pages).to all(include('Alex Smith'))
    expect(8.times.map { |index| pages.join(' ').scan("Evidence marker #{index}").size }).to all(eq(1))
  end

  def prompt(index: nil)
    attributes = prompt_attributes.merge(person_id: prompt_attributes[:person].id)
    attributes[:evidence_text] = "Evidence marker #{index}" if index
    Data.define(*attributes.keys).new(**attributes)
  end
end
