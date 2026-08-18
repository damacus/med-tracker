# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Delete medication confirmation', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    page.current_window.resize_to(390, 844)
    visit medications_path
  end

  it 'dismisses with Cancel and Escape without deleting the medication, restores focus, and reopens' do
    trigger = find("##{tenant_dom_id(medication)} button[aria-label='Delete medication']")
    trigger.click

    dialog = find('dialog[role="alertdialog"]')
    expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
    expect(page).to have_css('body.overflow-hidden')
    expect_dialog_actions_to_fill_footer(dialog)
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

  def expect_dialog_actions_to_fill_footer(dialog)
    widths = dialog.evaluate_script(<<~JS)
      (() => {
        const footer = this.querySelector('.flex.flex-col-reverse');
        const footerWidth = footer.getBoundingClientRect().width;
        return {
          footer: footerWidth,
          buttons: Array.from(footer.querySelectorAll('button')).map((button) => button.getBoundingClientRect().width)
        };
      })()
    JS

    expect(widths.fetch('buttons')).to all(be_within(1).of(widths.fetch('footer')))
  end
end
