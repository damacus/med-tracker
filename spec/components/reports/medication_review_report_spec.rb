require 'rails_helper'

RSpec.describe Components::Reports::MedicationReviewReport, type: :component do
  let(:content) do
    [
      'To discuss',
      'Reviewed',
      'Alex Smith',
      'Warfarin + Ibuprofen',
      'High source risk',
      'Public label evidence',
      'Why MedTracker included this',
      'Matched term: ibuprofen',
      'curated match',
      'Source instruction category: Label says avoid',
      'DailyMed',
      'Label version: 4 | Effective: 1 July 2026',
      'Retrieved: 9 July 2026',
      'Reviewed with Dr Taylor',
      'GP on 9 July 2026.',
      'Recorded outcome: expected as prescribed.',
      'Confirmed as prescribed.'
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

  it 'renders the grouped evidence, risk, source and practitioner outcome without querying records' do
    rendered = Nokogiri::HTML5(described_class.new(prompts: [reviewed_prompt]).call)

    expect(rendered.text).to include(*content)
    expect(rendered.at_css('a')['href']).to eq('https://example.test/evidence')
    expect(rendered.at_css('.risk-high')).to be_present
  end

  it 'renders the translated empty state and boundary for every supported locale' do
    %i[en cy es ga pt].each do |locale|
      rendered = I18n.with_locale(locale) do
        Nokogiri::HTML5(described_class.new(prompts: []).call)
      end

      expect(rendered.text).to include(
        I18n.t('reports.medication_review.empty_title', locale:),
        I18n.t('reports.medication_review.boundary', locale:)
      )
    end
  end

  it 'uses the report locale date format for practitioner outcomes' do
    rendered = I18n.with_locale(:es) do
      Nokogiri::HTML5(described_class.new(prompts: [reviewed_prompt]).call)
    end

    expect(rendered.text).to include('Revisado con Dr Taylor', 'GP el 9 de julio de 2026.')
  end

  def reviewed_prompt
    attributes = prompt_attributes.merge(person_id: prompt_attributes[:person].id)
    Data.define(*attributes.keys).new(**attributes)
  end
end
