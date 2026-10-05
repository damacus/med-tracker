require 'rails_helper'

RSpec.describe 'Removing medication stock', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:medication) { household_medication(medications(:paracetamol)) }

  before do
    driven_by(:playwright)
    sign_in(users(:admin))
  end

  it 'records a removal and displays its history' do
    visit medication_path(medication)
    click_link 'Remove stock'
    fill_in 'Quantity removed', with: '1'
    select 'Dropped', from: 'Reason'
    fill_in 'Note', with: 'Dropped during setup'
    capture_stock_removal('stock-removal-desktop')
    click_button 'Record removal'
    expect(page).to have_text('Stock removal recorded.')
    expect(medication.reload.current_supply).to eq(79)
    click_link 'Remove stock'
    expect(page).to have_text('Dropped during setup')
    expect(page).to have_text('Dropped')
  end

  it 'fits a mobile viewport and retains a failed submission' do
    page.current_window.resize_to(390, 844)
    visit medication_path(medication)
    click_link 'Remove stock'
    fill_in 'Quantity removed', with: '999'
    select 'Lost', from: 'Reason'
    fill_in 'Note', with: 'Checking stock'
    click_button 'Record removal'
    expect(page).to have_text('Not enough stock')
    expect(page).to have_field('Note', with: 'Checking stock')
    capture_stock_removal('stock-removal-mobile-error')
    viewport_width = page.evaluate_script('window.innerWidth')
    expect(page.evaluate_script('document.documentElement.scrollWidth')).to be <= viewport_width
  end

  def capture_stock_removal(name)
    return unless ENV['STOCK_REMOVAL_SCREENSHOTS'] == 'true'

    directory = Rails.root.join('docs/screenshots/issue-1982')
    FileUtils.mkdir_p(directory)
    page.driver.with_playwright_page do |browser_page|
      browser_page.screenshot(path: directory.join("#{name}.png").to_s)
    end
  end
end
