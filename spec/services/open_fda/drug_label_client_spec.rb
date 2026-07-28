# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::DrugLabelClient do
  it 'rejects non-positive worker counts' do
    expect { described_class.new(worker_count: 0) }
      .to raise_error(ArgumentError, 'worker_count must be positive')
  end

  it 'prefers a matching human monotherapy label over a combination product' do
    response = {
      'meta' => { 'last_updated' => '2026-07-10' },
      'results' => [
        label('combination', ['ACETAMINOPHEN', 'OXYCODONE HYDROCHLORIDE']),
        label('monotherapy', ['OXYCODONE HYDROCHLORIDE'])
      ]
    }
    result = client_with(response).labels_for(['oxycodone'])

    expect(result.fetch('results').sole.fetch('set_id')).to eq('monotherapy')
  end

  it 'requires every configured target in targeted interaction labels' do
    response = {
      'meta' => { 'last_updated' => '2026-07-10' },
      'results' => [label('doxazosin-label', ['DOXAZOSIN MESYLATE'])]
    }
    client = client_with(response)
    targeted_selection = [
      { 'term' => 'doxazosin', 'interaction_targets' => %w[ibuprofen acetaminophen] }
    ]

    result = client.labels_for_targeted(targeted_selection)

    expect(result.fetch('results').sole.fetch('set_id')).to eq('doxazosin-label')
    expect(client.requested_entries).to eq(targeted_selection)
  end

  it 'uses eight workers by default while preserving input order' do
    active_workers = maximum_workers = 0
    lock = Mutex.new
    client = instrumented_client do |entry|
      lock.synchronize do
        active_workers += 1
        maximum_workers = [maximum_workers, active_workers].max
      end
      sleep(0.02)
      response_for(entry.fetch('term'))
    ensure
      lock.synchronize { active_workers -= 1 }
    end

    result = client.labels_for(Array.new(16) { |index| "term-#{index}" })

    expect(maximum_workers).to eq(8)
    expect(result.fetch('results').pluck('set_id')).to eq(Array.new(16) { |index| "term-#{index}" })
  end

  it 'honours a smaller worker count' do
    active_workers = 0
    maximum_workers = 0
    lock = Mutex.new
    client = instrumented_client(worker_count: 2) do |entry|
      lock.synchronize do
        active_workers += 1
        maximum_workers = [maximum_workers, active_workers].max
      end
      sleep(0.01)
      response_for(entry.fetch('term'))
    ensure
      lock.synchronize { active_workers -= 1 }
    end

    client.labels_for(%w[first second third])

    expect(maximum_workers).to eq(2)
  end

  it 'returns an empty response without starting workers' do
    requested_entries = []
    client = instrumented_client do |entry|
      requested_entries << entry
      response_for(entry.fetch('term'))
    end

    expect(client.labels_for([])).to eq('meta' => {}, 'results' => [])
    expect(requested_entries).to be_empty
  end

  it 'retains the open and read timeouts for batch requests' do
    http = instance_double(Net::HTTP, get: successful_response(response_for('warfarin')))
    allow(Net::HTTP).to receive(:start) { |*, &block| block.call(http) }

    described_class.new(worker_count: 1).labels_for(['warfarin'])

    expect(Net::HTTP).to have_received(:start).with(
      'api.fda.gov', 443, use_ssl: true, open_timeout: 5, read_timeout: 20
    )
  end

  def client_with(response)
    client_class = Class.new(described_class) do
      attr_reader :requested_entries

      define_method(:concurrent_responses) do |entries|
        @requested_entries = entries
        [response]
      end
    end
    client_class.new
  end

  def instrumented_client(worker_count: nil, &request)
    client_class = Class.new(described_class) do
      define_method(:response_for) { |entry| request.call(entry) }
    end
    worker_count ? client_class.new(worker_count:) : client_class.new
  end

  def response_for(term)
    {
      'meta' => { 'last_updated' => '2026-07-10' },
      'results' => [label(term, [term])]
    }
  end

  def successful_response(body)
    response_class = Class.new(Net::HTTPSuccess) { define_method(:body) { JSON.generate(body) } }
    response_class.new('1.1', '200', 'OK')
  end

  def label(set_id, substances)
    {
      'set_id' => set_id,
      'openfda' => {
        'product_type' => ['HUMAN PRESCRIPTION DRUG'],
        'substance_name' => substances
      }
    }
  end
end
