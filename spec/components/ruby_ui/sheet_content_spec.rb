# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::SheetContent, type: :component do
  let(:named_sheet_component) do
    Class.new(Phlex::HTML) do
      def view_template
        render RubyUI::SheetContent.new(side: :right) do
          render(RubyUI::SheetTitle.new { 'Change password' })
          render(RubyUI::SheetDescription.new { 'Choose a new password.' })
        end
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
    rendered = render_inline(described_class.new { 'Sheet body' })
    close_button = rendered.at_css(%(button[aria-label="#{I18n.t('ruby_ui.common.close')}"]))

    expect(close_button).to be_present
    expect(close_button.at_css('svg[aria-hidden="true"]')).to be_present
    expect(close_button.css('.sr-only')).to be_empty
  end

  it 'renders reusable sheet naming and responsive sizing hooks' do
    rendered = render_inline(named_sheet_component.new)
    panel = rendered.at_css('[role="dialog"]')

    expect(panel['aria-label']).to be_nil
    expect(panel['class']).to include('max-w-md')
    expect(panel['class']).not_to include('max-w-[300px]')
    expect(panel.at_css('[data-ruby-ui-sheet-title]')).to be_present
    expect(panel.at_css('[data-ruby-ui-sheet-description]')).to be_present
  end
end
