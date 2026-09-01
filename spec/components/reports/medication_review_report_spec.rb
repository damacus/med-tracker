require 'rails_helper'

RSpec.describe Components::Reports::MedicationReviewReport, type: :component do
  let(:content) do
    [
      'To discuss',
      'Reviewed',
      'Alex Smith',
      'Warfarin + Ibuprofen',
      'High source risk',
      'High match confidence',
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

  it 'renders grouped evidence, risk and source details without querying records' do
    rendered = Nokogiri::HTML5(described_class.new(prompts: [reviewed_prompt]).call)

    expect(rendered.text).to include(*content)
    expect(rendered.at_css('a')['href']).to eq('https://example.test/evidence')
    expect(rendered.at_css('.risk-high')).to be_present
  end

  it 'renders the person heading and practitioner outcome' do
    rendered = Nokogiri::HTML5(described_class.new(prompts: [reviewed_prompt]).call)
    person_review = rendered.at_css('.person-review')

    expect(person_review.at_css('thead h2').text).to eq('Alex Smith')
    expect(person_review.at_css('thead .person-review-count')).to be_present
    expect(person_review.at_css('tbody .review-prompt')).to be_present
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

  it 'renders translated confidence values for populated reports in every supported locale' do
    %i[en cy es ga pt].each do |locale|
      rendered = I18n.with_locale(locale) do
        Nokogiri::HTML5(described_class.new(prompts: [reviewed_prompt]).call)
      end
      confidence = I18n.t('reports.medication_review.confidence_levels.high', locale:)

      expect(rendered.text).to include(
        I18n.t('reports.medication_review.confidence', locale:, value: confidence)
      )
    end
  end

  it 'keeps report locale keys, value types and interpolations in sync' do
    reference_contract = locale_contract(:en)

    %i[en cy es ga pt].each do |locale|
      expect(locale_contract(locale)).to eq(reference_contract), "locale contract drift for #{locale}"
    end
  end

  it 'renders person identity as a repeatable header for prompt rows' do
    rendered = Nokogiri::HTML5(described_class.new(prompts: Array.new(8, reviewed_prompt)).call)
    person_review = rendered.at_css('table.person-review')

    expect(person_review.at_css('thead h2').text).to eq('Alex Smith')
    expect(person_review.css('tbody tr').size).to eq(8)
    expect(person_review.css('tbody tr')).to all(
      satisfy { |row| row.text.include?('Warfarin + Ibuprofen') }
    )
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

  def locale_contract(locale)
    translations = YAML.safe_load_file(Rails.root.join("config/locales/#{locale}.yml"), aliases: true)
    translation_contract(translations.dig(locale.to_s, 'reports', 'medication_review'))
  end

  def translation_contract(value)
    case value
    when Hash
      value.transform_values { |child| translation_contract(child) }
    when Array
      value.map { |child| translation_contract(child) }
    when String
      [:scalar, value.scan(/%\{([^}]+)\}/).flatten.sort]
    else
      value.class.name
    end
  end
end
