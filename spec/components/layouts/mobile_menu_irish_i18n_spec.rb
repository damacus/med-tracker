# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileMenu, type: :component do
  describe 'i18n translations' do
    around do |example|
      I18n.with_locale(:ga) { example.run }
    end

    it 'renders mobile menu with Irish translations' do
      component = described_class.new

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Dún roghchlár')
      expect(rendered.to_html).to include('Daoine')
      expect(rendered.to_html).to include('Aimsitheoir Leigheas')
      expect(rendered.to_html).to include('Próifíl')
      expect(rendered.to_html).to include('Logáil amach')
    end
  end
end
