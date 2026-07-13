# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::CalendarPrev, type: :component do
  it 'hides the previous month icon from its labelled button' do
    rendered = render_inline(described_class.new)
    button = rendered.at_css(%(button[aria-label="#{I18n.t('ruby_ui.calendar.previous_month')}"]))

    expect(button.at_css('svg[aria-hidden="true"]')).to be_present
  end

  it 'hides the next month icon from its labelled button' do
    rendered = render_inline(RubyUI::CalendarNext.new)
    button = rendered.at_css(%(button[aria-label="#{I18n.t('ruby_ui.calendar.next_month')}"]))

    expect(button.at_css('svg[aria-hidden="true"]')).to be_present
  end
end
