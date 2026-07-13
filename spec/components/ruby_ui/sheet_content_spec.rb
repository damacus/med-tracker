# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::SheetContent, type: :component do
  let(:close_button_component) do
    Class.new(described_class) do
      def view_template
        close_button
      end
    end
  end

  it 'uses token-driven shell surfaces for the drawer' do
    rendered = render_inline(described_class.new(side: :right) { 'Sheet body' })
    html = rendered.to_html

    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).to include('bg-popover')
    expect(html).not_to include('bg-slate-950/12')
  end

  it 'names the close button and hides redundant close content' do
    rendered = render_inline(close_button_component.new)
    close_button = rendered.at_css(%(button[aria-label="#{I18n.t('ruby_ui.common.close')}"]))

    expect(close_button).to be_present
    expect(close_button.at_css('svg[aria-hidden="true"]')).to be_present
    expect(close_button.css('.sr-only')).to be_empty
  end
end
