# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::Pagination, type: :component do
  let(:pagy) do
    double( # rubocop:disable RSpec/VerifiedDoubles
      'Pagy',
      count: 25,
      from: 1,
      to: 10,
      pages: 3,
      page: 1,
      previous: nil,
      next: 2,
      series: [1, 2, 3]
    )
  end

  describe 'pagination info' do
    it 'renders the result count information' do
      rendered = render_inline(described_class.new(pagy: pagy))

      expect(rendered.text).to include('25')
      expect(rendered.text).to include('results')
    end

    it 'renders the from and to range' do
      rendered = render_inline(described_class.new(pagy: pagy))

      expect(rendered.text).to include('1')
      expect(rendered.text).to include('10')
    end
  end

  describe 'navigation' do
    it 'renders page number links' do
      rendered = render_inline(described_class.new(pagy: pagy))

      expect(rendered.css('nav[aria-label="Pagination"]')).to be_present
    end

    it 'disables previous button on first page' do
      rendered = render_inline(described_class.new(pagy: pagy))

      prev_span = rendered.css('span.sr-only').find { |s| s.text == 'Previous' }
      expect(prev_span).to be_present
      prev_container = prev_span.parent
      expect(prev_container['class']).to include('cursor-not-allowed')
    end

    it 'enables next button when there is a next page' do
      rendered = render_inline(described_class.new(pagy: pagy))

      next_links = rendered.css('a').select { |a| a.css('.sr-only').any? { |s| s.text == 'Next' } }
      expect(next_links).to be_present
    end
  end

  describe 'mobile pagination' do
    it 'renders previous and next buttons for mobile' do
      rendered = render_inline(described_class.new(pagy: pagy))

      expect(rendered.text).to include('Previous')
      expect(rendered.text).to include('Next')
    end
  end

  context 'when on the last page' do
    let(:pagy) do
      double( # rubocop:disable RSpec/VerifiedDoubles
        'Pagy',
        count: 25,
        from: 21,
        to: 25,
        pages: 3,
        page: 3,
        previous: 2,
        next: nil,
        series: [1, 2, 3]
      )
    end

    it 'disables next button on last page' do
      rendered = render_inline(described_class.new(pagy: pagy))

      next_span = rendered.css('span.sr-only').select { |s| s.text == 'Next' }
      next_container = next_span.last&.parent
      expect(next_container['class']).to include('cursor-not-allowed')
    end

    it 'enables previous button' do
      rendered = render_inline(described_class.new(pagy: pagy))

      prev_links = rendered.css('a').select { |a| a.css('.sr-only').any? { |s| s.text == 'Previous' } }
      expect(prev_links).to be_present
    end
  end

  context 'when there is only one page' do
    let(:pagy) do
      double( # rubocop:disable RSpec/VerifiedDoubles
        'Pagy',
        count: 5,
        from: 1,
        to: 5,
        pages: 1,
        page: 1,
        previous: nil,
        next: nil,
        series: [1]
      )
    end

    it 'does not render the pagination nav' do
      rendered = render_inline(described_class.new(pagy: pagy))

      expect(rendered.css('nav[aria-label="Pagination"]')).to be_empty
    end
  end

  describe 'search params preservation' do
    it 'includes search params in page links' do
      rendered = render_inline(described_class.new(
                                 pagy: pagy,
                                 search_params: { search: 'test', role: 'admin' }
                               ))

      page_links = rendered.css('a[href*="admin/users"]')
      page_links.each do |link|
        expect(link['href']).to include('search=test') if link['href']
      end
    end
  end

  describe 'accessibility' do
    it 'renders pagination nav with aria-label' do
      rendered = render_inline(described_class.new(pagy: pagy))

      nav = rendered.css('nav[aria-label="Pagination"]')
      expect(nav).to be_present
    end

    it 'renders sr-only text for previous and next buttons' do
      rendered = render_inline(described_class.new(pagy: pagy))

      sr_only = rendered.css('.sr-only').map(&:text)
      expect(sr_only).to include('Previous')
      expect(sr_only).to include('Next')
    end
  end
end
