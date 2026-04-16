# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication reorder workflow' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:medication) { medications(:aspirin) }

  before do
    sign_in admin
    # Ensure medication is low stock to trigger ordering UI
    medication.update!(current_supply: 5, reorder_threshold: 10)
  end

  it 'allows marking a medication as ordered and then received' do
    visit medication_path(medication)

    expect(page).to have_content('Low Stock Alert')
    expect(page).to have_content('Order')

    click_on 'Order'

    # The 'Order' action link should be gone after clicking it
    within "[data-testid='medication-content']" do
      expect(page).to have_no_link('Order')
    end

    click_on 'Received'

    expect(page).to have_content('Refill marked as received')
    expect(page).to have_content('Received')
    expect(medication.reload.reordered_at).to be_present
    
    # Check for Restock button within content area
    within "[data-testid='medication-content']" do
      expect(page).to have_content('Restock')
    end

    click_on 'Restock'

    # The dialog content is moved to the end of the body by the Stimulus controller
    # We can search by the unique header or just the fields
    expect(page).to have_content("Restock #{medication.name}")
    fill_in 'Quantity', with: 50
    click_on 'Refill'

    expect(page).to have_content('Inventory refilled successfully')
    expect(medication.reload.current_supply).to eq(55)
    expect(medication.reorder_status).to be_nil # Should be cleared after refill
  end
end
