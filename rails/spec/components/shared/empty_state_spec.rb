# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::EmptyState, type: :component do
  it 'renders the title and description' do
    rendered = render_inline(described_class.new(
                               title: 'No medications yet',
                               description: 'Add your first medication to get started.'
                             ))

    expect(rendered.text).to include('No medications yet')
    expect(rendered.text).to include('Add your first medication to get started.')
  end
end
