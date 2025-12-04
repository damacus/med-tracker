# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :accounts, :account_otp_keys, :users, :medicines, :dosages, :prescriptions, :people

  it 'loads the dashboard for a signed-in user' do
    sign_in(users(:john))

    visit dashboard_path

    within '[data-testid="dashboard"]' do
      aggregate_failures 'dashboard content' do
        expect(page).to have_content('Dashboard')
        expect(page).to have_link('Add Medicine', href: new_medicine_path)
        expect(page).to have_link('Add Person', href: new_person_path)
      end
    end
  end

  context 'when viewing prescription delete buttons' do
    context 'when user is an administrator' do
      it 'shows delete buttons for prescriptions' do
        sign_in(users(:admin))

        visit dashboard_path

        within '[data-testid="dashboard"]' do
          prescription = prescriptions(:active_prescription)
          within "#prescription_#{prescription.id}" do
            expect(page).to have_button('Delete')
          end
        end
      end
    end

    context 'when user is a doctor' do
      it 'shows delete buttons for prescriptions' do
        sign_in(users(:doctor))

        visit dashboard_path

        within '[data-testid="dashboard"]' do
          prescription = prescriptions(:active_prescription)
          within "#prescription_#{prescription.id}" do
            expect(page).to have_button('Delete')
          end
        end
      end
    end

    context 'when user is a carer' do
      it 'does not show delete buttons for prescriptions' do
        sign_in(users(:carer))

        visit dashboard_path

        within '[data-testid="dashboard"]' do
          expect(page).to have_no_button('Delete')
        end
      end
    end

    context 'when user is a parent' do
      it 'does not show delete buttons for prescriptions' do
        sign_in(users(:parent))

        visit dashboard_path

        within '[data-testid="dashboard"]' do
          expect(page).to have_no_button('Delete')
        end
      end
    end
  end
end
