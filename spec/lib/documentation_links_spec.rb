# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'DocumentationLinks' do
  def local_links(source)
    source.read.scan(/\[[^\]]+\]\(([^)]+)\)/).flatten.filter_map do |target|
      next if target.start_with?('http://', 'https://', '#')

      source.dirname.join(target.split('#', 2).first)
    end
  end

  it 'resolves links from the documentation entry points' do
    sources = [
      Rails.root.join('docs/index.md'),
      Rails.root.join('docs/testing.md'),
      Rails.root.join('docs/passkey-setup.md'),
      Rails.root.join('docs/families/quick-setup.md')
    ]
    missing = sources.flat_map { |source| local_links(source) }.reject(&:exist?)

    expect(missing).to be_empty, "Missing documentation targets: #{missing.join(', ')}"
  end

  it 'links the manual accessibility smoke-test checklist from contributor testing' do
    testing_guide = Rails.root.join('docs/testing.md').read

    expect(testing_guide).to include('accessibility-smoke-test.md')
  end

  it 'connects the family quick setup journey' do
    guide = Rails.root.join('docs/families/quick-setup.md').read

    expect(guide).to include(
      '../self-hosting.md',
      'adding-first-medicine.md',
      'taking-first-dose.md',
      '../user-management.md'
    )
  end
end
