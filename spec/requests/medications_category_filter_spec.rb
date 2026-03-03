# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications category filter' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'renders inventory category combobox with all option' do
    get medications_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('data-controller="ruby-ui--combobox"')
    expect(response.body).to include('name="category"')
    expect(response.body).to include(I18n.t('medications.index.all'))
  end

  it 'filters medications by category and keeps selected value' do
    get medications_path(category: 'Vitamin')

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('data-controller="ruby-ui--combobox"')
    expect(response.body).to include('value="Vitamin"')
    expect(response.body).to include('checked')
    expect(response.body).to include('Vitamin D')
    expect(response.body).to include('Vitamin C')
    expect(response.body).not_to include('Paracetamol')
  end

  it 'shows all medications when category is blank' do
    get medications_path(category: '')

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Paracetamol')
    expect(response.body).to include('Vitamin D')
  end
end
