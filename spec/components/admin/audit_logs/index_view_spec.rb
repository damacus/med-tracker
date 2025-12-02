# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::IndexView, type: :component do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:versions) do
    PaperTrail.request.whodunnit = admin.id
    person.update!(name: 'Updated Name')
    person.update!(email: 'new@example.com')
    PaperTrail::Version.order(created_at: :desc).limit(10)
  end

  after do
    PaperTrail.request.whodunnit = nil
  end

  describe 'initialization' do
    it 'accepts versions and filter_params' do
      view = described_class.new(versions: versions, filter_params: { item_type: 'Person' })

      expect(view.versions).to eq(versions)
      expect(view.filter_params).to eq({ item_type: 'Person' })
    end

    it 'defaults filter_params to empty hash' do
      view = described_class.new(versions: versions)

      expect(view.filter_params).to eq({})
    end
  end

  describe '#filters_active?' do
    subject(:view) { described_class.new(versions: versions, filter_params: filter_params) }

    context 'when no filters are set' do
      let(:filter_params) { {} }

      it 'returns false' do
        expect(view.send(:filters_active?)).to be(false)
      end
    end

    context 'when item_type filter is set' do
      let(:filter_params) { { item_type: 'Person' } }

      it 'returns true' do
        expect(view.send(:filters_active?)).to be(true)
      end
    end

    context 'when event filter is set' do
      let(:filter_params) { { event: 'update' } }

      it 'returns true' do
        expect(view.send(:filters_active?)).to be(true)
      end
    end
  end

  describe 'pagination' do
    subject(:view) do
      described_class.new(
        versions: versions,
        filter_params: {},
        current_page: current_page,
        total_count: total_count,
        per_page: per_page
      )
    end

    let(:current_page) { 1 }
    let(:per_page) { 50 }

    describe '#total_pages' do
      context 'when total_count is 0' do
        let(:total_count) { 0 }

        it 'returns 1' do
          expect(view.send(:total_pages)).to eq(1)
        end
      end

      context 'when total_count is less than per_page' do
        let(:total_count) { 25 }

        it 'returns 1' do
          expect(view.send(:total_pages)).to eq(1)
        end
      end

      context 'when total_count is greater than per_page' do
        let(:total_count) { 75 }

        it 'returns correct number of pages' do
          expect(view.send(:total_pages)).to eq(2)
        end
      end

      context 'when total_count is exactly per_page' do
        let(:total_count) { 50 }

        it 'returns 1' do
          expect(view.send(:total_pages)).to eq(1)
        end
      end
    end

    describe '#first_item_number' do
      let(:total_count) { 100 }

      context 'on page 1' do
        let(:current_page) { 1 }

        it 'returns 1' do
          expect(view.send(:first_item_number)).to eq(1)
        end
      end

      context 'on page 2' do
        let(:current_page) { 2 }

        it 'returns 51' do
          expect(view.send(:first_item_number)).to eq(51)
        end
      end
    end

    describe '#last_item_number' do
      context 'when on last page with partial results' do
        let(:total_count) { 75 }
        let(:current_page) { 2 }

        it 'returns total_count' do
          expect(view.send(:last_item_number)).to eq(75)
        end
      end

      context 'when on full page' do
        let(:total_count) { 100 }
        let(:current_page) { 1 }

        it 'returns per_page' do
          expect(view.send(:last_item_number)).to eq(50)
        end
      end
    end

    describe '#pagination_url' do
      let(:total_count) { 100 }

      it 'generates URL with page parameter' do
        url = view.send(:pagination_url, 2)
        expect(url).to include('page=2')
      end

      it 'preserves filter parameters' do
        view_with_filters = described_class.new(
          versions: versions,
          filter_params: { item_type: 'User' },
          current_page: 1,
          total_count: 100,
          per_page: 50
        )
        url = view_with_filters.send(:pagination_url, 2)
        expect(url).to include('item_type=User')
        expect(url).to include('page=2')
      end
    end
  end
end
