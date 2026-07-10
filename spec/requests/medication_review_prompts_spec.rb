# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication review prompts' do
  fixtures :all

  let(:household) { households(:fixture_household) }
  let(:person) { people(:john) }
  let(:user) { users(:admin) }
  let(:warfarin) do
    household.medications.create!(
      name: 'Warfarin 1mg tablets',
      location: locations(:home),
      dose_amount: 1,
      dose_unit: 'tablet',
      current_supply: 28,
      reorder_threshold: 7
    )
  end

  before do
    sign_in(user)
    assign_medicine(warfarin, dose_amount: 1, dose_unit: 'tablet')
    assign_medicine(medications(:ibuprofen), dose_amount: 200, dose_unit: 'mg')
  end

  it 'shows a dedicated evidence-review page for an adult household member' do
    get medication_review_prompts_path

    expect(response).to have_http_status(:success)
    expect(response.body).to include('Medicine reviews')
    expect(response.body).to include('John Doe')
    expect(response.body).to include('Warfarin 1mg tablets')
    expect(response.body).to include('Ibuprofen')
    expect(response.body).to include('Public medicine-label evidence')
    expect(response.body).to include('A reviewed rule identifies ibuprofen as the interacting medicine.')
    expect(response.body).to include('Matched term: ibuprofen (curated match)')
    expect(response.body).to include('Source instruction category: No instruction category assigned')
    expect(response.body).to include('Label version 4, effective 1 July 2026')
    expect(response.body).not_to include('interaction warning')

    rendered_page = Capybara.string(response.body)
    expect(rendered_page).to have_css('main', count: 1)
    expect(rendered_page).to have_link('Export review PDF', href: medication_review_report_path)
    expect(rendered_page.find('[data-review-prompt-id] > div')[:class]).to include('medication-review-layout')
  end

  it 'discloses filtered noise and reveals it only when requested' do
    create_low_confidence_evidence

    get medication_review_prompts_path

    expect(response.body).to include('1 lower-confidence review item hidden to reduce noise')
    expect(response.body).not_to include('Test-only lower-confidence evidence')

    get medication_review_prompts_path, params: { show_hidden: '1' }

    expect(response.body).to include('Test-only lower-confidence evidence')
  end

  it 'records who reviewed the evidence and the practitioner outcome' do
    get medication_review_prompts_path
    prompt = MedicationReviewPrompt.visible_by_default.sole

    patch medication_review_prompt_path(prompt), params: {
      medication_review_prompt: {
        status: 'expected_prescribed_combination',
        practitioner_name: 'Dr Taylor',
        practitioner_role: 'GP',
        reviewed_on: '2026-07-09',
        review_note: 'Confirmed this combination was expected as prescribed.'
      }
    }

    expect(response).to redirect_to(medication_review_prompts_path)
    expect(prompt.reload).to have_attributes(
      status: 'expected_prescribed_combination',
      practitioner_name: 'Dr Taylor',
      practitioner_role: 'GP',
      reviewed_on: Date.new(2026, 7, 9),
      review_note: 'Confirmed this combination was expected as prescribed.',
      reviewed_by_membership: HouseholdMembership.find_by!(account: user.person.account, household: household)
    )
  end

  it 'does not expose prompts for a person outside the member access scope' do
    get medication_review_prompts_path
    post '/logout'
    sign_in(users(:parent))

    get medication_review_prompts_path

    expect(response).to have_http_status(:success)
    expect(response.body).not_to include('John Doe')
    expect(response.body).not_to include('Warfarin 1mg tablets')
  end

  it 'rejects access from a minor household member' do
    post '/logout'
    sign_in(users(:minor_patient_user))

    get medication_review_prompts_path

    expect(response).to redirect_to(root_path)
    expect(flash[:alert]).to eq('You are not authorized to perform this action.')
  end

  it 'downloads an appointment-ready no-store PDF' do
    get medication_review_prompts_path

    get medication_review_report_path, params: { person_id: person.id, status: 'needs_review' }

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/pdf')
    expect(response.headers['Cache-Control']).to include('no-store')
    expected_filename = "medtracker-medication-review-#{Date.current.iso8601}.pdf"
    expect(response.headers['Content-Disposition']).to include(expected_filename)
    expect(response.body).to start_with('%PDF')
    text_fragments = response.body.scan(/<([0-9A-Fa-f]+)>/).flatten.map { |hex| [hex].pack('H*') }.join
    expect(text_fragments).to include('Matched term: ibuprofen (curated match)')
    expect(text_fragments).to include('Label version: 4')
  end

  it 'does not export an inaccessible person filter' do
    get medication_review_prompts_path
    post '/logout'
    sign_in(users(:parent))

    get medication_review_report_path, params: { person_id: person.id }

    expect(response).to have_http_status(:ok)
    expect(response.body).to start_with('%PDF')
  end

  def assign_medicine(medication, dose_amount:, dose_unit:)
    PersonMedication.create!(
      household: household,
      person: person,
      medication: medication,
      dose_amount: dose_amount,
      dose_unit: dose_unit,
      administration_kind: 'as_needed'
    )
  end

  def create_low_confidence_evidence
    MedicationReviewEvidenceRecord.create!(
      source_name: 'DailyMed',
      source_record_id: 'low-confidence-request-spec',
      source_url: 'https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=low-confidence-request-spec',
      retrieved_on: Date.new(2026, 7, 9),
      product_name: 'Warfarin Sodium',
      label_section: 'Drug Interactions',
      evidence_text: 'Test-only lower-confidence evidence.',
      risk_level: 'unknown',
      match_confidence: 'low',
      match_status: 'reviewed_pair',
      candidate_terms: %w[warfarin],
      interacting_terms: %w[ibuprofen]
    )
  end
end
