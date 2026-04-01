# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::FinderView, type: :component do
  it 'renders medication search translations for the Stimulus controller' do
    rendered = render_inline(described_class.new)
    root = rendered.at_css('[data-controller="medication-search"]')

    expect(root).to be_present

    payload = JSON.parse(root['data-medication-search-translations-value'])

    expect(payload).to include(
      'loading' => I18n.t('medications.finder.loading'),
      'results_title' => I18n.t('medications.finder.results_title'),
      'dmd_code' => I18n.t('medications.finder.dmd_code')
    )
    expect(payload.fetch('result_count')).to include(
      'one' => I18n.t('medications.finder.result_count.one'),
      'other' => I18n.t('medications.finder.result_count.other')
    )
  end
end
