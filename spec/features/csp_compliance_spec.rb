# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'CSP Compliance', type: :system do
  describe 'inline styles' do
    it 'does not use inline styles in RubyUI components' do
      # Check dropdown menu content
      dropdown_html = RubyUI::DropdownMenuContent.new.call
      expect(dropdown_html).not_to match(/style=/)
      expect(dropdown_html).not_to match(/pointer-events:auto/)

      # Check alert dialog content
      alert_dialog_html = RubyUI::AlertDialogContent.new.call
      expect(alert_dialog_html).not_to match(/style=/)
      expect(alert_dialog_html).to match(/pointer-events-auto/)

      # Check sheet content
      sheet_html = RubyUI::SheetContent.new.call
      expect(sheet_html).not_to match(/style=/)
      expect(sheet_html).to match(/pointer-events-auto/)
    end
  end
end
