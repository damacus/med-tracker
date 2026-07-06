# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Household Redirects' do
  fixtures :accounts, :people, :users

  describe 'GET /' do
    context 'when user is not signed in' do
      it 'redirects to login path' do
        get root_path

        expect(response).to redirect_to('/login')
        expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      end
    end

    context 'when user is signed in with an active household membership' do
      it 'redirects to the household dashboard' do
        household, = household_membership_for(users(:jane))
        sign_in(users(:jane))

        get root_path

        expect(response).to redirect_to("/households/#{household.slug}/dashboard")
      end
    end

    context 'when user is signed in but has no active household membership' do
      it 'redirects to login path and clears session' do
        user = users(:jane)
        sign_in(user)
        # Ensure no active memberships
        user.person.account.household_memberships.update_all(status: :inactive)

        get root_path

        expect(response).to redirect_to('/login')
        expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      end
    end
  end

  def household_membership_for(user)
    household = user.person.household
    membership = household.household_memberships.find_or_create_by!(account: user.person.account, person: user.person)
    membership.update!(role: :member, status: :active)

    [household, membership]
  end
end
