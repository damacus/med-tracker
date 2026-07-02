# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Select, type: :component do
  it 'renders a native select with M3 styling' do
    rendered = render_inline(described_class.new(name: 'dose[unit]', id: 'dose_unit')) do |component|
      component.option(value: 'mg', selected: true) { 'mg' }
    end

    select = rendered.at_css('select#dose_unit[name="dose[unit]"]')

    aggregate_failures do
      expect(select).to be_present
      expect(select['class']).to include('rounded-shape-sm')
      expect(select['class']).to include('border-outline')
      expect(select['class']).to include('bg-transparent')
      expect(select['class']).to include('text-foreground')
      expect(select.at_css('option[selected]')['value']).to eq('mg')
    end
  end

  it 'supports compact sizing for filter and admin forms' do
    rendered = render_inline(described_class.new(name: 'event', id: 'event', size: :sm))

    expect(rendered.at_css('select#event')['class']).to include('h-9')
  end

  it 'preserves required, disabled, and caller supplied classes' do
    rendered = render_inline(
      described_class.new(name: 'person_id', id: 'person_id', required: true, disabled: true, class: 'max-w-xs')
    )

    select = rendered.at_css('select#person_id')

    aggregate_failures do
      expect(select).to have_attribute('required')
      expect(select).to have_attribute('disabled')
      expect(select['class']).to include('max-w-xs')
    end
  end

  it 'merges form-field data attributes with caller supplied data actions' do
    select = rendered_select_with_data

    aggregate_failures do
      expect(select['data-ruby-ui--form-field-target']).to eq('input')
      expect(select['data-filter-form-target']).to eq('input')
      expect(select['data-action']).to include('change->ruby-ui--form-field#onChange')
      expect(select['data-action']).to include('invalid->ruby-ui--form-field#onInvalid')
      expect(select['data-action']).to include('change->filter-form#submit')
    end
  end

  def rendered_select_with_data
    render_inline(
      described_class.new(
        name: 'item_type',
        id: 'item_type',
        data: { action: 'change->filter-form#submit', filter_form_target: 'input' }
      )
    ).at_css('select#item_type')
  end
end
