# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::UserRow, type: :component do
  fixtures :accounts, :people, :users

  let(:user) { users(:admin) }
  let(:current_user) { users(:admin) }

  describe 'actions cell layout' do
    it 'uses gap-2 instead of space-x-2 for flex layout' do
      rendered = render_inline(described_class.new(user: user, current_user: current_user))

      actions_cell = rendered.css('td').last
      expect(actions_cell['class']).to include('gap-2')
      expect(actions_cell['class']).not_to include('space-x-2')
    end

    it 'uses flex and justify-end for proper alignment' do
      rendered = render_inline(described_class.new(user: user, current_user: current_user))

      actions_cell = rendered.css('td').last
      expect(actions_cell['class']).to include('flex')
      expect(actions_cell['class']).to include('justify-end')
    end
  end
end
