require 'rails_helper'

RSpec.describe 'Medication stock removals' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:medication) { household_medication(medications(:paracetamol)) }
  let(:path) { "/households/#{default_request_household_slug}/medications/#{medication.id}/stock-removals" }
  let(:attributes) do
    { quantity: '1', reason: 'dropped', note: 'Private removal note', submission_id: SecureRandom.uuid }
  end

  before { sign_in(users(:admin)) }

  it 'renders a labelled form and a link from the medicine' do
    get medication_path(medication)
    expect(response.body).to include('Remove stock')
    get "#{path}/new"
    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Quantity removed', 'Reason', 'Note', 'Recent removals')
  end

  it 'records attributable history and redirects after a successful removal' do
    expect { post path, params: { stock_removal: attributes } }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:see_other)
    expect(medication.reload.current_supply).to eq(79)
    event = RemoveMedicationStockService.history(medication).last
    expect(event.whodunnit).to eq(users(:admin).id.to_s)
    expect(event.actor_membership_id).to be_present
    get "#{path}/new"
    expect(response.body).to include('Private removal note', 'Dropped', '79')
  end

  it 'retains invalid input with a validation error' do
    post path, params: { stock_removal: attributes.merge(quantity: '999') }
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('Not enough stock', 'Private removal note', '999')
    expect(medication.reload.current_supply).to eq(80)
  end

  it 'rejects missing fields without an exception' do
    post path, params: { stock_removal: { note: 'Incomplete' } }
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(80)
  end

  it 'rejects members without inventory management permission' do
    sign_in(users(:parent))
    create(:person_medication, person: users(:parent).person, medication: medication)
    get "#{path}/new"
    expect(response).to redirect_to(root_path)
    post path, params: { stock_removal: attributes }
    expect(response).to redirect_to(root_path)
    expect(medication.reload.current_supply).to eq(80)
  end

  it 'rejects a medicine outside the household' do
    foreign_household = Household.create!(name: 'Other', slug: 'other-stock-removal')
    foreign_medication = Medication.create!(name: 'Foreign private medicine', household: foreign_household,
                                            location: foreign_household.locations.create!(name: 'Other'),
                                            current_supply: 5)
    post path.sub("/#{medication.id}/", "/#{foreign_medication.id}/"), params: { stock_removal: attributes }
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('Foreign private medicine')
    expect(foreign_medication.reload.current_supply).to eq(5)
  end

  it 'rejects tampered stock sources' do
    post path, params: { stock_removal: attributes.merge(dosage_id: '-1') }
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(80)
  end

  it 'filters private removal details from request diagnostics' do
    filter = ActiveSupport::ParameterFilter.new(Rails.application.config.filter_parameters)
    expect(filter.filter('stock_removal' => attributes)['stock_removal']).to eq('[FILTERED]')
  end
end
