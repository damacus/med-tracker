# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::MetricCard, type: :component do
  it 'renders a block link wrapper when href is provided' do
    rendered = render_inline(
      described_class.new(title: 'People', value: 5, icon_type: 'users', href: '/people')
    )

    link = rendered.at_css('a[href="/people"]')
    expect(link).to be_present
    expect(link['class']).to include('block')
    expect(link['class']).to include('h-full')
    expect(link['class']).not_to include('h-9')
  end

  it 'renders a non-link wrapper when href is omitted' do
    rendered = render_inline(
      described_class.new(title: 'People', value: 5, icon_type: 'users')
    )

    expect(rendered.css('a')).to be_empty
    expect(rendered.css('div.h-full')).to be_present
  end

  it 'renders badge text when provided' do
    rendered = render_inline(
      described_class.new(title: 'Compliance', value: '85%', icon_type: 'check', badge: 'Needs review')
    )

    expect(rendered.text).to include('Needs review')
  end

  it 'applies warning variant classes' do
    rendered = render_inline(
      described_class.new(title: 'No Carers', value: 2, icon_type: 'activity', variant: :warning)
    )
    html = rendered.to_html

    expect(html).to include('bg-warning-container')
    expect(html).to include('border-warning')
    expect(html).to include('text-on-warning-container')
  end

  it 'adds custom data attributes to the value element' do
    rendered = render_inline(
      described_class.new(
        title: 'Total Users',
        value: 10,
        icon_type: 'users',
        value_data_attr: { metric_value: 10 }
      )
    )

    expect(rendered.css('[data-metric-value="10"]')).to be_present
  end

  it 'renders compact layout classes when requested' do
    rendered = render_inline(
      described_class.new(title: 'Next Dose', value: '14:47', icon_type: 'clock', layout: :compact)
    )
    html = rendered.to_html
    card = rendered.css('div').find do |node|
      node[:class]&.include?('min-h-[7rem]')
    end

    expect(html).to include('min-h-[7rem]')
    expect(html).to include('p-4')
    expect(html).to include('text-2xl')
    expect(html).not_to include('md:hover:scale-[1.02]')
    expect(card[:class]).not_to include('h-full')
  end
end
