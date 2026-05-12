# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::MetricCard, type: :component do
  let(:active_schedules_icon_path) do
    [
      'M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 ',
      '23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 ',
      '22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 ',
      '720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 ',
      '720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z'
    ].join
  end

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

  it 'renders the active schedules icon path' do
    rendered = render_inline(
      described_class.new(title: 'Active Schedules', value: 10, icon_type: 'active_schedules')
    )

    expect(rendered.at_css('svg')['viewbox']).to eq('0 -960 960 960')
    expect(rendered.at_css("path[d='#{active_schedules_icon_path}']")).to be_present
  end
end
