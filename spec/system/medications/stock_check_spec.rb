# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Household stock check', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:paracetamol) { household_medication(medications(:paracetamol)) }
  let(:aspirin) { household_medication(medications(:aspirin)) }
  let(:calpol) { household_medication(medications(:calpol)) }

  before do
    driven_by(:playwright)
    sign_in(admin)
  end

  it 'selects and applies several supply amendments together' do
    visit medications_path
    click_link 'Stock check'

    expect(page).to have_current_path(%r{/medications/stock_check\z})
    expect(page).to have_button('Apply 0 amendments', disabled: true)

    check "stock_check_medication_#{paracetamol.id}"
    within batch_row(paracetamol) do
      fill_in "stock_check_quantity_#{paracetamol.id}", with: '74'
    end

    check "stock_check_medication_#{aspirin.id}"
    within batch_row(aspirin) do
      click_button 'Set to zero'
      expect(page).to have_text('Out of stock')
    end

    visible_batch_ids = all("[data-stock-check-target='batchRow']", visible: true).pluck('data-medication-id')
    expect(visible_batch_ids).to eq([paracetamol.id.to_s, aspirin.id.to_s])
    expect(page).to have_text('2 medicines · total net change -31 units')
    click_button 'Apply 2 amendments'

    expect(page).to have_text('Updated the remaining supply for 2 medicines.')
    expect(paracetamol.reload.current_supply).to eq(74)
    expect(aspirin.reload.current_supply).to eq(0)
  end

  it 'fits the stock amendment workspace within a mobile viewport' do
    page.current_window.resize_to(390, 844)
    visit medications_path
    click_link 'Stock check'
    check "stock_check_medication_#{calpol.id}"

    viewport_width = page.evaluate_script('window.innerWidth')
    content_width = page.evaluate_script('document.documentElement.scrollWidth')

    expect(content_width).to be <= viewport_width
  end

  def batch_row(medication)
    "[data-stock-check-target='batchRow'][data-medication-id='#{medication.id}']"
  end
end
