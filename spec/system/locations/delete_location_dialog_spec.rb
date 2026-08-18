# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Delete location confirmation', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:location) { locations(:grandmas) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    page.current_window.resize_to(390, 844)
    visit locations_path
  end

  it 'cancels, dismisses, restores focus, and reopens without deleting the location' do
    trigger = find("##{tenant_dom_id(location)} button[aria-label='Delete location']")
    trigger.click

    dialog = find('dialog[open][role="alertdialog"]', text: 'Delete Location')
    expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
    expect_dialog_actions_to_fill_footer(dialog)
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
