# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Delete location confirmation', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:location) { locations(:grandmas) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    visit locations_path
  end

  it 'cancels, dismisses, restores focus, and reopens without deleting the location' do
    trigger = find("##{tenant_dom_id(location)} button[aria-label='Delete location']")
    trigger.click

    dialog = find('dialog[open][role="alertdialog"]', text: 'Delete Location')
    expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
    click_button I18n.t('locations.index.delete_dialog.cancel')

    expect(page).to have_no_css('dialog[open][role="alertdialog"]')
    expect(location.reload).to be_present
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('Delete location')

    trigger.click
    find('body').send_keys(:escape)

    expect(page).to have_no_css('dialog[open][role="alertdialog"]')
    expect(location.reload).to be_present
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('Delete location')

    trigger.click
    expect(page).to have_css('dialog[open][role="alertdialog"]')
  end
end
