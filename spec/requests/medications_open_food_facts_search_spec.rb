# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'GET /medication-finder/search via Open Food Facts' do
  fixtures :accounts, :people, :users

  let(:doctor) { users(:doctor) }
  let(:doctor_account) { accounts(:dr_jones) }

  def login_as_doctor
    post '/login', params: { email: doctor_account.email, password: 'password' }
  end

  it 'returns a supplement result when the barcode is not in dm+d' do
    login_as_doctor

    search = instance_double(
      NhsDmd::Search,
      call: NhsDmd::Search::Result.new(
        results: [
          NhsDmd::SearchResult.new(
            code: nil,
            name: 'Wellman Original',
            description: 'Daily multivitamin food supplement',
            display: 'Wellman Original (Vitabiotics) 30 tablets',
            system: 'https://world.openfoodfacts.org',
            concept_class: 'Supplement',
            category: 'Supplement',
            package_size: '30 tablets',
            package_quantity: 30,
            package_unit: 'tablet'
          )
        ],
        error: nil,
        resolved_query: 'Wellman Original (Vitabiotics) 30 tablets',
        barcode: '5021265221301'
      )
    )
    allow(NhsDmd::Search).to receive(:new).and_return(search)

    get medication_finder_search_path(format: :json), params: { q: '5021265221301' }

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body['results']).to contain_exactly(
      a_hash_including(
        'code' => nil,
        'name' => 'Wellman Original',
        'description' => 'Daily multivitamin food supplement',
        'display' => 'Wellman Original (Vitabiotics) 30 tablets',
        'concept_class' => 'Supplement',
        'category' => 'Supplement',
        'package_size' => '30 tablets',
        'package_quantity' => 30,
        'package_unit' => 'tablet'
      )
    )
    expect(response.parsed_body['barcode']).to eq('5021265221301')
  end

  it 'returns supplement metadata for non-medical text search results' do
    login_as_doctor

    search = instance_double(
      NhsDmd::Search,
      call: NhsDmd::Search::Result.new(
        results: [
          NhsDmd::SearchResult.new(
            code: nil,
            barcode: '5021265221301',
            name: 'Wellman Original',
            description: 'Daily multivitamin food supplement',
            display: 'Wellman Original (Vitabiotics) 30 tablets',
            system: 'https://world.openfoodfacts.org',
            concept_class: 'Supplement',
            category: 'Supplement',
            package_size: '30 tablets',
            package_quantity: 30,
            package_unit: 'tablet'
          )
        ],
        error: nil
      )
    )
    allow(NhsDmd::Search).to receive(:new).and_return(search)

    get medication_finder_search_path(format: :json), params: { q: 'wellman' }

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body['results']).to contain_exactly(
      a_hash_including(
        'code' => nil,
        'barcode' => '5021265221301',
        'name' => 'Wellman Original',
        'description' => 'Daily multivitamin food supplement',
        'display' => 'Wellman Original (Vitabiotics) 30 tablets',
        'category' => 'Supplement',
        'package_size' => '30 tablets',
        'package_quantity' => 30,
        'package_unit' => 'tablet',
        'source_label' => 'Open Food Facts'
      )
    )
  end
end
