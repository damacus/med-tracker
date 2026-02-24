# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  describe 'i18n translations' do
    around do |example|
      I18n.with_locale(:ga) { example.run }
    end

    it 'renders quick actions with Irish translations' do
      component = described_class.new

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Gníomhartha Tapa')
      expect(rendered.to_html).to include('Cuir Leigheas Leis')
    end
  end
end
