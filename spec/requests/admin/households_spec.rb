# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin household settings' do
  fixtures :all

  let(:household) { Household.find_by!(slug: default_request_household_slug) }
  let(:owner) { users(:admin) }
  let(:member) { users(:jane) }

  it 'allows a household owner to rename the current household' do
    sign_in(owner)

    patch admin_household_path, params: { household: { name: 'Damacus Household' } }

    expect(response).to redirect_to(edit_admin_household_path)
    expect(household.reload.name).to eq('Damacus Household')
  end

  it 'renders validation errors for a blank household name' do
    sign_in(owner)

    patch admin_household_path, params: { household: { name: '' } }

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('prevented this household from being saved')
    expect(response.body).to include('blank')
  end

  it 'denies household members from renaming households' do
    sign_in(member)

    patch admin_household_path, params: { household: { name: 'Member Rename' } }

    expect(response).to redirect_to(root_path)
    expect(household.reload.name).not_to eq('Member Rename')
  end
end
