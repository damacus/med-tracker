# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AmbiguousPersonAccessGrants::IndexView, type: :component do
  let(:pagy) do
    Class.new do
      attr_reader :count, :from, :to, :pages, :page, :previous, :next

      def initialize
        @count = 25
        @from = 1
        @to = 10
        @pages = 3
        @page = 1
        @previous = nil
        @next = 2
      end
    end.new
  end

  it 'renders accessible navigation links when more than one page exists' do
    rendered = render_inline(described_class.new(grants: [], pagy: pagy))

    expect(rendered.css('nav[aria-label="Pagination"]')).to be_present
    expect(rendered.css('a').pluck('href')).to include(a_string_including('page=2'))
    expect(rendered.text).to include('Previous', 'Next')
  end
end
