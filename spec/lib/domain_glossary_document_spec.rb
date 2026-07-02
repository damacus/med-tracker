# frozen_string_literal: true

require 'rails_helper'

class DomainGlossaryDocument
  def glossary = Rails.root.join('docs/glossary.md').read

  def readme = Rails.root.join('README.md').read
end

RSpec.describe DomainGlossaryDocument do
  subject(:document) { described_class.new }

  it 'defines the canonical ubiquitous language terms' do
    canonical_terms = [
      '### Person',
      '### Schedule',
      '### PersonMedication',
      '### MedicationTake',
      '### Supply',
      '### Carer'
    ]

    expect(document.glossary).to include(*canonical_terms)
  end

  it 'documents naming guidance and is discoverable from the README' do
    expect(document.glossary).to include('Prefer these names in new code, UI copy, documentation, and tests.')
    expect(document.glossary).to include(
      'Avoid introducing new patient, individual, plan, routine, stock, or dose-record'
    )
    expect(document.readme).to include('[Glossary](docs/glossary.md)')
  end
end
