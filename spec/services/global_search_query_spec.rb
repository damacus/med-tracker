# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearchQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages,
           :schedules, :carer_relationships, :person_medications, :medication_takes

  subject(:results) { described_class.new(user: user, query: query, limit: limit).call }

  let(:user) { users(:jane) }
  let(:query) { 'vitamin' }
  let(:limit) { 5 }

  it 'returns normalized results across the configured record types' do
    types = results.map(&:type)

    expect(types).to include('medication')
    expect(types).to include('person_medication')
    expect(results).to all(
      have_attributes(
        type: a_kind_of(String),
        title: a_kind_of(String),
        subtitle: a_kind_of(String),
        path: a_kind_of(String),
        score: a_kind_of(Integer)
      )
    )
  end

  it 'ranks exact title matches ahead of broader matches' do
    ranked = described_class.new(user: users(:damacus), query: 'Vitamin D', limit: 10).call

    expect(ranked.first).to have_attributes(type: 'medication', title: 'Vitamin D')
  end

  it 'uses friendly medication display names in result titles' do
    medication = medications(:calpol)
    medication.update!(
      name: 'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets',
      friendly_name: 'Movicol Paediatric Plain'
    )

    ranked = described_class.new(user: users(:damacus), query: 'Movicol', limit: 10).call

    expect(ranked.first).to have_attributes(type: 'medication', title: 'Movicol Paediatric Plain')
  end

  it 'scores exact command title matches as exact matches' do
    command_match = described_class.new(user: users(:damacus), query: 'Inventory', limit: 10).call.first

    expect(command_match).to have_attributes(type: 'command', title: 'Inventory', score: 100)
  end

  it 'uses policy scopes for people and schedules' do
    scoped = described_class.new(user: users(:jane), query: 'John', limit: 10).call

    expect(scoped.map(&:title)).not_to include('John Doe')
    expect(scoped.map(&:path)).not_to include(
      Rails.application.routes.url_helpers.person_path('test-household', people(:john))
    )
  end

  it 'returns authorized command shortcuts when the query is blank' do
    command_results = described_class.new(user: users(:jane), query: '', limit: 10).call

    expect(command_results.map(&:type)).to all(eq('command'))
    expect(command_results.map(&:title)).to include('Add medication', 'People')
  end

  it 'respects the requested result limit' do
    limited_results = described_class.new(user: users(:jane), query: 'a', limit: 2).call

    expect(limited_results.size).to eq(2)
  end

  it 'respects the requested result limit for blank command shortcuts' do
    limited_results = described_class.new(user: users(:jane), query: '', limit: 2).call

    expect(limited_results.size).to eq(2)
  end

  it 'does not search records for one-character queries' do
    short_query_results = described_class.new(user: users(:jane), query: 'v', limit: 10).call

    expect(short_query_results.map(&:type).uniq).to eq(['command'])
    expect(short_query_results.map(&:title)).not_to include('Vitamin D')
  end

  it 'does not search medication take records, notes, audit logs, or user accounts' do
    sensitive_queries = ['For back pain', users(:john).email_address, 'taken']

    sensitive_queries.each do |sensitive_query|
      result_titles = described_class.new(user: users(:damacus), query: sensitive_query, limit: 10).call.map(&:title)
      expect(result_titles).to be_empty
    end
  end
end
