# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Request sign-in helper' do
  fixtures :accounts, :people, :users

  it 'switches accounts before household-scoped requests' do
    sign_in(users(:carer))

    household = Household.create_with_owner!(
      name: 'Request Helper Household',
      owner_account: users(:admin).person.account,
      owner_person_attributes: {
        name: 'Request Helper Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )

    sign_in(users(:admin))
    get "/households/#{household.slug}/offline"

    expect(response).to have_http_status(:ok)
  end

  it 'redirects authenticated root requests to the active household dashboard' do
    household = Household.create_with_owner!(
      name: 'Root Household',
      owner_account: users(:admin).person.account,
      owner_person_attributes: {
        name: 'Root Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )

    sign_in(users(:admin))
    get root_path

    expect(response).to redirect_to("/households/#{household.slug}/dashboard")
  end
end
