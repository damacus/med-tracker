# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::UserRow, type: :component do
  fixtures :accounts, :people, :users

  let(:user) { users(:admin) }
  let(:current_user) { users(:admin) }

  describe 'actions cell layout' do
    it 'does not apply flex directly to the table cell' do
      rendered = render_inline(described_class.new(user: user, current_user: current_user))

      actions_cell = rendered.css('td').last
      expect(actions_cell['class']).to include('text-right')
      expect(actions_cell['class']).not_to include('flex')
    end

    it 'wraps actions in an inner flex div with gap-2 and justify-end' do
      rendered = render_inline(described_class.new(user: user, current_user: current_user))

      actions_div = rendered.css('td').last.css('div').first
      expect(actions_div).to be_present
      expect(actions_div['class']).to include('flex')
      expect(actions_div['class']).to include('gap-2')
      expect(actions_div['class']).to include('justify-end')
    end
  end
end
