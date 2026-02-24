# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  describe 'i18n translations' do
    it 'renders quick actions with Irish translations' do
      I18n.locale = :ga
      component = described_class.new

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Gníomhartha Tapa')
      expect(rendered.to_html).to include('Cuir Leigheas Leis')
    end
  end
end
