# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Location member removal', :browser do
  fixtures :accounts, :people, :users, :locations

  let(:admin) { users(:admin) }
  let(:location) { locations(:grandmas) }
  let(:member) { people(:john) }

  before do
    driven_by(:playwright)
    LocationMembership.create!(location: location, person: member)
    sign_in(admin)
    page.current_window.resize_to(390, 844)
    visit location_path(location)
    wait_for_removal_dialog
  end

  it 'keeps removal discoverable and restores focus after keyboard cancellation' do
    trigger = find("button[aria-label='Remove member']")
    trigger.send_keys(:enter)

    dialog = find('dialog[open][role="alertdialog"]', text: 'Remove Member')
    expect(dialog).to have_text(member.name)
    click_button I18n.t('locations.show.remove_member.cancel')

    expect(page).to have_no_css('dialog[open][role="alertdialog"]')
    expect(page).to have_css("button[aria-label='Remove member']:focus")
  end

  def wait_for_removal_dialog
    page.driver.with_playwright_page do |browser|
      browser.wait_for_function(<<~JS)
        () => {
          const trigger = document.querySelector("button[aria-label='Remove member']");
          const dialog = trigger?.closest('[data-controller~="ruby-ui--alert-dialog"]');
          return dialog && window.Stimulus?.getControllerForElementAndIdentifier(dialog, 'ruby-ui--alert-dialog');
        }
      JS
    end
  end
end
