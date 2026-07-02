# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules

  subject(:rendered) { render_inline(described_class.new(schedules: [schedules(:john_paracetamol)])) }

  it 'keeps mobile schedule cards' do
    mobile_list = rendered.css('[data-testid="schedules-mobile-list"]')

    expect(mobile_list).to be_present
    expect(mobile_list.text).to include('Paracetamol')
    expect(mobile_list.text).to include('John Doe')
  end

  it 'keeps the desktop RubyUI table' do
    desktop_table = rendered.at_css('[data-testid="schedules-desktop-table"]')

    expect(desktop_table.at_css('.relative.w-full.overflow-auto')).to be_present
    expect(desktop_table.at_css('table')['class']).to include('caption-bottom')
    expect(desktop_table.at_css('thead')['class']).to include('[&_tr]:border-b')
    expect(desktop_table.at_css('tbody')['class']).to include('[&_tr:last-child]:border-0')
  end
end
