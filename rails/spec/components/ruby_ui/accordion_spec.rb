# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Accordion, type: :component do
  let(:accordion_component) do
    Class.new(Phlex::HTML) do
      def view_template
        render RubyUI::Accordion.new do
          render RubyUI::AccordionItem.new(id: 'profile-section', open: true) do
            render RubyUI::AccordionTrigger.new(
              id: 'profile-section-trigger',
              controls: 'profile-section-content',
              expanded: true
            ) { 'Profile' }
            render RubyUI::AccordionContent.new(
              id: 'profile-section-content',
              labelledby: 'profile-section-trigger',
              open: true
            ) { 'Profile settings' }
          end
        end
      end
    end
  end

  it 'connects an accessible trigger to its content' do
    rendered = render_inline(accordion_component.new)

    trigger = rendered.at_css('button')
    content = rendered.at_css('[data-ruby-ui--accordion-target="content"]')

    expect(trigger['aria-expanded']).to eq('true')
    expect(trigger['aria-controls']).to eq(content['id'])
    expect(content['aria-labelledby']).to eq(trigger['id'])
  end
end
