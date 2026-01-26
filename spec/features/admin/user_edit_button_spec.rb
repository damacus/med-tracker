# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin User Edit Button', type: :system do
  fixtures :all

  let(:admin) { users(:damacus) }
  let(:target_user) { users(:john) }

  before do
    driven_by(:playwright)
    sign_in(admin)
    visit admin_users_path
  end

  it 'navigates to edit page when Edit button is clicked' do
    within "[data-user-id='#{target_user.id}']" do
      expect(page).to have_content(target_user.name)

      # Click the Edit button
      click_link 'Edit'
    end

    # Verify we navigated to the edit page
    expect(page).to have_current_path(edit_admin_user_path(target_user))
    expect(page).to have_content('Edit User')
    expect(page).to have_field('Email address', with: target_user.email_address)
  end

  it 'Edit button is accessible and has proper styling' do
    within "[data-user-id='#{target_user.id}']" do
      edit_button = find('a', text: 'Edit')

      # Verify it's a link (for navigation)
      expect(edit_button.tag_name).to eq('a')

      # Verify it has href attribute
      expect(edit_button[:href]).to include("/admin/users/#{target_user.id}/edit")

      # Verify it has button styling (outline variant)
      expect(edit_button[:class]).to include('border')
    end
  end
end
