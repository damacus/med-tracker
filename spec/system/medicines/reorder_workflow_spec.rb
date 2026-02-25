# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicine reorder workflow' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:medicine) { medicines(:aspirin) }

  before do
    sign_in admin
    # Ensure medicine is low stock to trigger ordering UI
    medicine.update!(current_supply: 5, reorder_threshold: 10)
  end

  it 'allows marking a medicine as ordered and then received' do
    visit medicine_path(medicine)

    expect(page).to have_content('Low Stock Alert')
    expect(page).to have_link('Mark as Ordered')

    click_link 'Mark as Ordered'

    expect(page).to have_content('Refill marked as ordered')
    expect(page).to have_content('Ordered')
    expect(page).to have_link('Mark as Received')
    expect(page).to have_no_link('Mark as Ordered')

    click_link 'Mark as Received'

    expect(page).to have_content('Refill marked as received')
    expect(page).to have_content('Received')
    expect(medicine.reload.reordered_at).to be_present
    expect(page).to have_button('Complete Refill')

    click_button 'Complete Refill'

    # The dialog content is moved to the end of the body by the Stimulus controller
    # We can search by the unique header or just the fields
    expect(page).to have_content("Refill #{medicine.name}")
    fill_in 'Quantity', with: 50
    click_button 'Refill'

    expect(page).to have_content('Inventory refilled successfully')
    expect(medicine.reload.current_supply).to eq(55)
    expect(medicine.reorder_status).to be_nil # Should be cleared after refill
  end
end
