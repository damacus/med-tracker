# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Reports::FilterForm, type: :component do
  it 'hides the icon from the labelled apply filters button' do
    rendered = render_inline(
      described_class.new(
        action_path: '/reports',
        people: [],
        selected_person_id: nil,
        start_date: nil,
        end_date: nil
      )
    )
    apply_button = rendered.at_css(%(button[aria-label="#{I18n.t('reports.index.apply_filters_aria_label')}"]))

    expect(apply_button).to be_present
    expect(apply_button.at_css('svg[aria-hidden="true"]')).to be_present
  end
end
