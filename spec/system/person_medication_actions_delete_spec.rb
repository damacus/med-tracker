# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medication actions delete confirmation', :browser do
  fixtures :accounts, :people, :users, :locations, :medications, :person_medications

  let(:admin) { users(:admin) }
  let(:person_medication) { person_medications(:jane_vitamin_d) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    page.current_window.resize_to(390, 844)
    visit person_path(person_medication.person)
  end

  it 'dismisses Delete before its Actions menu and leaves the medication unchanged' do
    within("##{tenant_dom_id(person_medication)}") do
      find("[data-testid='person-medication-actions-#{person_medication.id}']").click
      find("[data-testid='delete-person-medication-#{person_medication.id}']").click
    end

    dialog = find('dialog[open][role="alertdialog"]', text: 'Remove Medication')
    expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
    expect(page).to have_no_css(
      "[data-testid='person-medication-actions-menu-#{person_medication.id}']", visible: :visible
    )

    find('body').send_keys(:escape)

    expect(page).to have_no_css('dialog[open][role="alertdialog"]')
    expect(person_medication.reload).to be_present
    expect(page).to have_css("[data-testid='person-medication-actions-menu-#{person_medication.id}']:not(.hidden)")
    expect(page.evaluate_script('document.activeElement.dataset.testid')).to eq(
      "delete-person-medication-#{person_medication.id}"
    )

    find('body').send_keys(:escape)

    expect(page).to have_no_css(
      "[data-testid='person-medication-actions-menu-#{person_medication.id}']", visible: :visible
    )
    expect(page.evaluate_script('document.activeElement.dataset.testid')).to eq(
      "person-medication-actions-#{person_medication.id}"
    )
  end
end
