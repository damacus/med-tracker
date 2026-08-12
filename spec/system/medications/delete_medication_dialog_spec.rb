# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Delete medication confirmation', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    visit medications_path
  end

  it 'dismisses with Cancel and Escape without deleting the medication, restores focus, and reopens' do
    trigger = find("##{tenant_dom_id(medication)} button[aria-label='Delete medication']")
    trigger.click

    dialog = find('dialog[role="alertdialog"]')
    expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
    expect(page).to have_css('body.overflow-hidden')
    click_button I18n.t('medications.index.delete_dialog.cancel')

    expect(page).to have_no_css('[role="alertdialog"]')
    expect(page).to have_no_css('body.overflow-hidden')
    expect(medication.reload).to be_present
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('Delete medication')

    trigger.click
    expect(page).to have_css('[role="alertdialog"]')

    find('body').send_keys(:escape)

    expect(page).to have_no_css('[role="alertdialog"]')
    expect(medication.reload).to be_present
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('Delete medication')

    trigger.click
    expect(page).to have_css('[role="alertdialog"]')
  end
end
