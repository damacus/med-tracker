# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::SearchResult do
  describe '#to_h' do
    it 'includes a valid HTTPS PIL URL' do
      result = described_class.new(
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        pil_url: 'https://www.medicines.org.uk/emc/product/13866/pil'
      )

      expect(result.to_h[:pil_url]).to eq('https://www.medicines.org.uk/emc/product/13866/pil')
    end

    it 'filters unsafe and malformed PIL URLs' do
      urls = [
        'javascript:alert(1)',
        'http://www.medicines.org.uk/emc/product/13866/pil',
        'not a url'
      ]

      expect(
        urls.map do |url|
          described_class.new(
            code: '39720311000001101',
            display: 'Aspirin 300mg tablets',
            system: 'https://dmd.nhs.uk',
            pil_url: url
          ).to_h[:pil_url]
        end
      ).to all(be_nil)
    end
  end
end
