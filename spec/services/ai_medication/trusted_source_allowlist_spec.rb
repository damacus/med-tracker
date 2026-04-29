# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::TrustedSourceAllowlist do
  subject(:allowlist) { described_class.new }

  it 'allows configured trusted medication guidance URLs' do
    expect(allowlist.allowed?(calpol_sixplus_url)).to be(true)
    expect(allowlist.allowed?('https://www.medicines.org.uk/emc/product/13866/pil')).to be(true)
  end

  it 'rejects lookalike and unrelated URLs' do
    expect(allowlist.allowed?("https://www.calpol.co.uk.evil.example#{calpol_sixplus_path}")).to be(false)
    expect(allowlist.allowed?('https://example.com/calpol-sixplus-oral-suspension-paracetamol')).to be(false)
    expect(allowlist.allowed?('javascript:alert(1)')).to be(false)
  end

  def calpol_sixplus_url
    "https://www.calpol.co.uk#{calpol_sixplus_path}"
  end

  def calpol_sixplus_path
    '/our-products/calpol-sixplus-oral-suspension-paracetamol'
  end
end
