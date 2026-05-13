# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::FinderView, type: :component do
  it 'renders medication search translations for the Stimulus controller' do
    payload = finder_translations_payload

    expect(payload).to include(
      'loading' => I18n.t('medications.finder.loading'),
      'resultsTitle' => I18n.t('medications.finder.results_title'),
      'dmdCode' => I18n.t('medications.finder.dmd_code'),
      'updateStock' => 'Update stock',
      'pilLink' => I18n.t('medications.finder.pil_link')
    )
    expect(payload.fetch('resultCount')).to include(
      'one' => I18n.t('medications.finder.result_count.one'),
      'other' => I18n.t('medications.finder.result_count.other')
    )
  end

  it 'renders restock modal translations for the Stimulus controller' do
    payload = finder_translations_payload

    expect(payload).to include(
      'confirmRestock' => I18n.t('medications.finder.confirm_restock'),
      'restockQuantity' => I18n.t('medications.finder.restock_quantity'),
      'restockSubmit' => I18n.t('medications.finder.restock_submit')
    )
  end

  def finder_translations_payload
    rendered = render_inline(described_class.new)
    root = rendered.at_css('[data-controller="medication-search"]')

    expect(root).to be_present

    JSON.parse(root['data-medication-search-translations-value'])
  end
end
