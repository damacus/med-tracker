# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships

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

  describe '#total_pages' do
    def build_view(total_count:, per_page: 50)
      described_class.new(versions: versions, total_count: total_count, per_page: per_page)
    end

    it 'returns 1 when total_count is 0' do
      expect(build_view(total_count: 0).send(:total_pages)).to eq(1)
    end

    it 'returns 1 when total_count is less than per_page' do
      expect(build_view(total_count: 25).send(:total_pages)).to eq(1)
    end

    it 'returns correct number of pages when total_count exceeds per_page' do
      expect(build_view(total_count: 75).send(:total_pages)).to eq(2)
    end

    it 'returns 1 when total_count equals per_page' do
      expect(build_view(total_count: 50).send(:total_pages)).to eq(1)
    end
  end

  describe '#first_item_number' do
    def build_view(current_page:)
      described_class.new(versions: versions, current_page: current_page, total_count: 100, per_page: 50)
    end

    it 'returns 1 when on page 1' do
      expect(build_view(current_page: 1).send(:first_item_number)).to eq(1)
    end

    it 'returns 51 when on page 2' do
      expect(build_view(current_page: 2).send(:first_item_number)).to eq(51)
    end
  end

  describe '#last_item_number' do
    def build_view(current_page:, total_count:)
      described_class.new(versions: versions, current_page: current_page, total_count: total_count, per_page: 50)
    end

    it 'returns total_count when on last page with partial results' do
      expect(build_view(current_page: 2, total_count: 75).send(:last_item_number)).to eq(75)
    end

    it 'returns per_page when on full page' do
      expect(build_view(current_page: 1, total_count: 100).send(:last_item_number)).to eq(50)
    end
  end

  describe '#pagination_url' do
    it 'generates URL with page parameter' do
      view = described_class.new(versions: versions, current_page: 1, total_count: 100, per_page: 50)
      expect(view.send(:pagination_url, 2)).to include('page=2')
    end

    it 'preserves filter parameters' do
      view = described_class.new(
        versions: versions,
        filter_params: { item_type: 'User' },
        current_page: 1,
        total_count: 100,
        per_page: 50
      )
      url = view.send(:pagination_url, 2)
      expect(url).to include('item_type=User')
      expect(url).to include('page=2')
    end
  end
end
