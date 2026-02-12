# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Heading Order Accessibility', type: :system do
  fixtures :accounts, :people, :users

  let(:user) { users(:carer) }
  let(:admin_user) { users(:admin) }

  def extract_heading_levels(page)
    page.all('h1, h2, h3, h4, h5, h6', visible: true).map do |heading|
      heading.tag_name.upcase
    end
  end

  def validate_heading_sequence(levels)
    return true if levels.empty?

    current_level = 0
    levels.each_with_index do |level, _index|
      level_num = level.gsub(/[^\d]/, '').to_i

      if current_level.zero?
        expect(level_num).to eq(1), "Page should start with H1, but starts with #{level}"
        current_level = 1
      elsif level_num > current_level
        expect(level_num).to eq(current_level + 1),
                             "Heading skips from H#{current_level} to #{level} (expected H#{current_level + 1})"
        current_level = level_num
      else
        current_level = level_num
      end
    end
  end

  describe 'Dashboard page' do
    before do
      sign_in(user)
    end

    it 'has proper heading hierarchy' do
      levels = extract_heading_levels(page)
      expect(levels).not_to be_empty
      expect(levels.first).to eq('H1')
      validate_heading_sequence(levels)
    end

    it 'does not skip from h1 to h3' do
      levels = extract_heading_levels(page)
      h1_indices = levels.each_index.select { |i| levels[i] == 'H1' }
      h1_indices.each do |h1_idx|
        next_headings = levels[(h1_idx + 1)..]
        next if next_headings.empty?

        first_non_h1 = next_headings.find { |h| h != 'H1' }
        next unless first_non_h1

        expect(first_non_h1).not_to eq('H3'), 'Found h1 followed by h3 without h2'
      end
    end
  end

  describe 'Admin Dashboard page' do
    before do
      sign_in(admin_user)
      visit '/admin'
    end

    it 'has proper heading hierarchy' do
      levels = extract_heading_levels(page)
      expect(levels).not_to be_empty
      expect(levels.first).to eq('H1')
      validate_heading_sequence(levels)
    end

    it 'does not skip from h1 to h3' do
      levels = extract_heading_levels(page)
      h1_indices = levels.each_index.select { |i| levels[i] == 'H1' }
      h1_indices.each do |h1_idx|
        next_headings = levels[(h1_idx + 1)..]
        next if next_headings.empty?

        first_non_h1 = next_headings.find { |h| h != 'H1' }
        next unless first_non_h1

        expect(first_non_h1).not_to eq('H3'), 'Found h1 followed by h3 without h2'
      end
    end
  end

  describe 'Medicines index page' do
    before do
      sign_in(user)
      visit '/medicines'
    end

    it 'has proper heading hierarchy' do
      levels = extract_heading_levels(page)
      expect(levels).not_to be_empty
      expect(levels.first).to eq('H1')
      validate_heading_sequence(levels)
    end
  end

  describe 'People index page' do
    before do
      sign_in(user)
      visit '/people'
    end

    it 'has proper heading hierarchy' do
      levels = extract_heading_levels(page)
      expect(levels).not_to be_empty
      expect(levels.first).to eq('H1')
      validate_heading_sequence(levels)
    end
  end

  describe 'Admin Users index page' do
    before do
      sign_in(admin_user)
      visit '/admin/users'
    end

    it 'has proper heading hierarchy' do
      levels = extract_heading_levels(page)
      expect(levels).not_to be_empty
      expect(levels.first).to eq('H1')
      validate_heading_sequence(levels)
    end
  end
end
