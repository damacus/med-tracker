# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Nlm::RxClassClient do
  it 'rejects non-positive worker counts' do
    expect { described_class.new(worker_count: -1) }
      .to raise_error(ArgumentError, 'worker_count must be positive')
  end

  it 'keeps public ingredient, EPC, and chemical class data while excluding mechanism classes' do
    entries = client_with(rxclass_response).entries_for(['phenelzine'])

    expect(entries.sole).to include(
      'selection_term' => 'phenelzine',
      'rxcui' => '8123',
      'ingredient_name' => 'phenelzine',
      'classes' => contain_exactly(
        { 'id' => 'N1', 'name' => 'Monoamine Oxidase Inhibitor', 'type' => 'EPC' },
        { 'id' => 'N3', 'name' => 'Hydrazines', 'type' => 'CHEM' }
      )
    )
  end

  it 'preserves response order with a small bounded pool' do
    client = instrumented_client(worker_count: 2) do |term|
      sleep(term == 'slow' ? 0.03 : 0.005)
      rxclass_response(term)
    end

    entries = client.entries_for(%w[slow fast last])

    expect(entries.pluck('ingredient_name')).to eq(%w[slow fast last])
  end

  it 'stops requesting queued entries after the first failure' do
    failure = RuntimeError.new('NLM unavailable')
    requested_terms = []
    client = instrumented_client(worker_count: 1) do |term|
      requested_terms << term
      raise failure if term == 'failure'

      rxclass_response(term)
    end

    expect { client.entries_for(%w[failure queued]) }.to raise_error(failure)
    expect(requested_terms).to eq(['failure'])
  end

  it 'propagates the first recorded concurrent failure' do
    first_worker = Queue.new
    first_failure = RuntimeError.new('first failure')
    second_failure = RuntimeError.new('second failure')
    client = instrumented_client(worker_count: 2) do |term|
      if term == 'first'
        first_worker << Thread.current
        raise first_failure
      end
      first_worker.pop.join
      raise second_failure
    end

    expect { client.entries_for(%w[first second]) }.to raise_error(first_failure)
  end

  it 'joins in-flight requests before propagating a failure' do
    both_started = Queue.new
    running_started = Queue.new
    release_request = Queue.new
    failure = RuntimeError.new('first failure')
    client = joining_client(both_started, running_started, release_request, failure)

    operation = Thread.new { captured_error { client.entries_for(%w[failure running]) } }
    2.times { both_started.pop }

    expect(operation).to be_alive
    release_request << true
    expect(operation.value).to equal(failure)
  end

  it 'returns an empty result without starting workers' do
    requested_terms = []
    client = instrumented_client(worker_count: 2) do |term|
      requested_terms << term
      rxclass_response(term)
    end

    expect(client.entries_for([])).to eq([])
    expect(requested_terms).to be_empty
  end

  it 'retains the open and read timeouts for batch requests' do
    http = instance_double(Net::HTTP, get: successful_response(rxclass_response))
    allow(Net::HTTP).to receive(:start) { |*, &block| block.call(http) }

    described_class.new(worker_count: 1).entries_for(['phenelzine'])

    expect(Net::HTTP).to have_received(:start).with(
      'rxnav.nlm.nih.gov', 443, use_ssl: true, open_timeout: 5, read_timeout: 20
    )
  end

  it 'is at least twenty percent faster with four delayed workers than with one' do
    terms = Array.new(8) { |index| "term-#{index}" }
    request = lambda do |term|
      sleep(0.02)
      rxclass_response(term)
    end

    serial_duration, serial_entries = measure { instrumented_client(worker_count: 1, &request).entries_for(terms) }
    parallel_duration, parallel_entries = measure { instrumented_client(worker_count: 4, &request).entries_for(terms) }

    warn format(
      'External I/O delay: serial=%<serial>.4fs parallel=%<parallel>.4fs',
      serial: serial_duration,
      parallel: parallel_duration
    )
    expect(parallel_entries).to eq(serial_entries)
    expect(parallel_duration).to be < (serial_duration * 0.8)
  end

  def client_with(response)
    client_class = Class.new(described_class) do
      define_method(:responses_for) { |_terms| [response] }
    end
    client_class.new
  end

  def instrumented_client(worker_count:, &request)
    client_class = Class.new(described_class) do
      define_method(:request_json) do |uri|
        term = URI.decode_www_form(uri.query).to_h.fetch('drugName')
        request.call(term)
      end
    end
    client_class.new(worker_count:)
  end

  def joining_client(started, running_started, release, failure)
    instrumented_client(worker_count: 2) do |term|
      started << term
      if term == 'failure'
        running_started.pop
        raise failure
      end
      running_started << true
      release.pop
      rxclass_response(term)
    end
  end

  def captured_error
    yield
  rescue StandardError => e
    e
  end

  def measure
    started_at = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    result = yield
    [Process.clock_gettime(Process::CLOCK_MONOTONIC) - started_at, result]
  end

  def successful_response(body)
    response_class = Class.new(Net::HTTPSuccess) { define_method(:body) { JSON.generate(body) } }
    response_class.new('1.1', '200', 'OK')
  end

  def rxclass_response(ingredient_name = 'phenelzine')
    {
      'rxclassDrugInfoList' => {
        'rxclassDrugInfo' => [
          drug_info('8123', ingredient_name, 'N1', 'Monoamine Oxidase Inhibitor', 'EPC'),
          drug_info('8123', ingredient_name, 'N2', 'Monoamine Oxidase Inhibitors', 'MOA'),
          drug_info('8123', ingredient_name, 'N3', 'Hydrazines', 'CHEM')
        ]
      }
    }
  end

  def drug_info(rxcui, ingredient_name, class_id, class_name, class_type)
    {
      'minConcept' => { 'rxcui' => rxcui, 'name' => ingredient_name, 'tty' => 'IN' },
      'rxclassMinConceptItem' => { 'classId' => class_id, 'className' => class_name, 'classType' => class_type },
      'relaSource' => 'DAILYMED'
    }
  end
end
