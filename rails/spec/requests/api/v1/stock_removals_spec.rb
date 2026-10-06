require 'rails_helper'

RSpec.describe 'API v1 stock removals' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:medication) { household_medication(medications(:paracetamol)) }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/medications/#{medication.id}/stock_removals" }
  let(:attributes) do
    { quantity: '1.25', reason: 'dropped', note: 'Dropped at home', submission_id: SecureRandom.uuid }
  end

  before do
    login
    medication
  end

  def stock_dosage(source)
    source.dosage_records.create!(amount: 1, unit: 'tablet', frequency: 'daily', current_supply: 10,
                                  default_max_daily_doses: 4, default_min_hours_between_doses: 4,
                                  default_dose_cycle: :daily)
  end

  def remove(**changes)
    post path, params: { stock_removal: attributes.merge(changes) }, headers: headers, as: :json
  end

  it 'removes decimal stock once and returns attributable history without a take' do
    expect { remove }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:created)
    expect(medication.reload.current_supply).to eq(BigDecimal('78.75'))
    expect(response.parsed_body.fetch('data')).to include('quantity' => '1.25', 'reason' => 'dropped',
                                                          'note' => 'Dropped at home', 'remaining_quantity' => '78.75')
    expect(response.parsed_body.dig('data', 'actor_membership_id')).to be_present
  end

  it 'returns the same removal on replay and rejects changed submission facts' do
    remove
    original = response.parsed_body
    expect { remove }.not_to(change { RemoveMedicationStockService.history(medication).count })
    expect(response.parsed_body).to eq(original)
    expect(medication.reload.current_supply).to eq(BigDecimal('78.75'))
    remove(quantity: '2')
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(BigDecimal('78.75'))
  end

  it 'rejects invalid decimal values and insufficient stock without domain changes' do
    ['0', '-1', '0.001', 'NaN', '999', '1e1', '+1', ' 1', 1.25].each do |quantity|
      expect { remove(quantity: quantity) }.not_to change(PaperTrail::Version, :count)
      expect(response).to have_http_status(:unprocessable_content)
      expect(medication.reload.current_supply).to eq(80)
    end
  end

  it 'rejects unsupported reasons, oversized notes and invalid submission identities' do
    [{ reason: 'private-invalid' }, { note: 'x' * 1001 }, { submission_id: 'bad' }].each do |change|
      remove(**change)
      expect(response).to have_http_status(:unprocessable_content)
      expect(medication.reload.current_supply).to eq(80)
      expect(response.body).not_to include('private-invalid')
    end
  end

  it 'rejects a dosage belonging to another medication' do
    other = create(:medication, household: medication.household, location: medication.location)
    dosage = stock_dosage(other)
    remove(dosage_id: dosage.id.to_s)
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(80)
    expect(dosage.reload.current_supply).to eq(10)
  end

  it 'removes from the selected dosage stock and refreshes the medication total' do
    dosage = stock_dosage(medication)
    remove(dosage_id: dosage.id.to_s)
    expect(response).to have_http_status(:created)
    expect(dosage.reload.current_supply).to eq(BigDecimal('8.75'))
    expect(medication.reload.current_supply).to eq(BigDecimal('8.75'))
    expect(response.parsed_body.dig('data', 'dosage_id')).to eq(dosage.id.to_s)
  end

  it 'returns bounded removal history in newest-first order' do
    remove
    first_id = response.parsed_body.dig('data', 'id')
    remove(submission_id: SecureRandom.uuid)
    second_id = response.parsed_body.dig('data', 'id')
    get path, params: { per_page: 1 }, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).to eq([second_id])
    expect(response.parsed_body.fetch('meta')).to include('page' => 1, 'per_page' => 1, 'total_count' => 2)
    get path, params: { per_page: 1, page: 2 }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data').pluck('id')).to eq([first_id])
  end

  it 'checks current inventory permission before returning a cached response' do
    headers['Idempotency-Key'] = SecureRandom.uuid
    remove
    expect(response).to have_http_status(:created)
    membership = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    membership.household.household_memberships.find_by!(account: accounts(:john_doe))
              .update!(role: :owner, status: :active)
    membership.update!(role: :member)
    remove
    expect(response).to have_http_status(:forbidden)
    expect(response.body).not_to include('Dropped at home')
    get path, headers: headers, as: :json
    expect(response).to have_http_status(:forbidden)
  end

  it 'does not disclose or change another household medication' do
    foreign = create(:medication, household: create(:household))
    post path.sub("/#{medication.id}/", "/#{foreign.id}/"),
         params: { stock_removal: attributes }, headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(foreign.name)
    expect(RemoveMedicationStockService.history(foreign)).to be_empty
  end

  it 'rolls stock back if the removal audit cannot be saved' do
    allow(Audit::VersionEvent).to receive(:record!).and_raise(ActiveRecord::RecordInvalid.new(PaperTrail::Version.new))
    remove
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(80)
    expect(RemoveMedicationStockService.history(medication)).to be_empty
  end

  it 'accepts the medication portable identity and caps oversized pages' do
    portable_path = path.sub("/#{medication.id}/", "/#{medication.portable_id}/")
    post portable_path, params: { stock_removal: attributes }, headers: headers, as: :json
    expect(response).to have_http_status(:created)
    get portable_path, params: { per_page: 1000 }, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('meta', 'per_page')).to eq(100)
  end

  it 'requires authentication for stock removal and history' do
    post path, params: { stock_removal: attributes }, as: :json
    expect(response).to have_http_status(:unauthorized)
    get path, as: :json
    expect(response).to have_http_status(:unauthorized)
    expect(RemoveMedicationStockService.history(medication)).to be_empty
  end
end
