# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::SearchResult do
  describe '#to_h' do
    it 'includes optional medicine detail fields' do
      result = described_class.new(
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        **medicine_detail_fields
      )

      expect(result.to_h).to include(medicine_detail_fields)
    end

    it 'includes a valid HTTPS PIL URL' do
      result = described_class.new(
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        pil_url: 'https://www.medicines.org.uk/emc/product/13866/pil',
        spc_url: 'https://www.medicines.org.uk/emc/product/13866/smpc'
      )

      expect(result.to_h[:pil_url]).to eq('https://www.medicines.org.uk/emc/product/13866/pil')
      expect(result.to_h[:spc_url]).to eq('https://www.medicines.org.uk/emc/product/13866/smpc')
    end

    it 'filters unsafe and malformed guidance URLs' do
      urls = [
        'javascript:alert(1)',
        'http://www.medicines.org.uk/emc/product/13866/pil',
        'not a url'
      ]

      expect(urls.flat_map { |url| guidance_url_values(url) }).to all(be_nil)
    end
  end

  def medicine_detail_fields
    {
      name: 'Aspirin 300mg tablets',
      description: 'Pain relief medicine',
      directions: 'Take with water',
      warnings: 'Do not exceed the stated dose',
      category: 'Analgesic',
      package_size: '32 tablets'
    }
  end

  def guidance_url_values(url)
    described_class.new(
      code: '39720311000001101',
      display: 'Aspirin 300mg tablets',
      system: 'https://dmd.nhs.uk',
      pil_url: url,
      spc_url: url
    ).to_h.slice(:pil_url, :spc_url).values
  end
end
