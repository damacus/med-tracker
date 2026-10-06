# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile tabs', :browser do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  before do
    driven_by(:playwright)
    login_as(user)
    page.current_window.resize_to(320, 844)
    visit profile_path
    page.execute_script('document.documentElement.style.fontSize = "32px"')
  end

  it 'keeps enlarged tab labels inside controls and supports keyboard navigation' do
    all('[data-testid="profile-section-tab"]').each do |tab|
      geometry = tab.evaluate_script(<<~JS)
        ({
          clientWidth: this.clientWidth,
          scrollWidth: this.scrollWidth,
          clientHeight: this.clientHeight,
          scrollHeight: this.scrollHeight
        })
      JS
      expect(geometry.fetch('scrollWidth')).to be <= geometry.fetch('clientWidth')
      expect(geometry.fetch('scrollHeight')).to be <= geometry.fetch('clientHeight')
    end

    first_tab = first('[data-testid="profile-section-tab"]')
    first_tab.send_keys(:right)

    expect(find('[data-profile-section="security"]')['aria-selected']).to eq('true')
    expect(find_by_id('profile-security-panel')).to be_visible
  end
end
