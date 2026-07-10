# frozen_string_literal: true

require 'net/http'

module OpenFda
  class DrugLabelClient
    class NotFound < StandardError; end

    ENDPOINT = 'https://api.fda.gov/drug/label.json'
    OPEN_TIMEOUT = 5
    READ_TIMEOUT = 20
    SEARCH_RESULT_LIMIT = 20
    WORKER_COUNT = 8

    def labels(limit:)
      request_json(request_uri(search: '_exists_:drug_interactions', limit: Integer(limit))).fetch('results')
    end

    def labels_for(terms)
      entries = terms.map { |term| { 'term' => term, 'interaction_targets' => [] } }
      responses = concurrent_responses(entries)
      {
        'meta' => responses.first.fetch('meta'),
        'results' => responses.zip(entries).map { |response, entry| select_label(response, entry.fetch('term')) }
      }
    end

    def labels_for_targeted(entries)
      responses = concurrent_responses(entries)
      {
        'meta' => responses.first.fetch('meta'),
        'results' => responses.zip(entries).map do |response, entry|
          select_label(response, entry.fetch('term'))
        end
      }
    end

    private

    def response_for(entry)
      term = entry.fetch('term')
      searches_for(term, entry.fetch('interaction_targets')).each do |search|
        return request_json(request_uri(search: search, limit: SEARCH_RESULT_LIMIT, sort: 'effective_time:desc'))
      rescue NotFound
        next
      end

      raise "openFDA returned no drug-interaction label for #{term.inspect}"
    end

    def select_label(response, term)
      results = response.fetch('results')
      candidates = results.select { |label| human_label?(label) && substance_matches?(label, term) }
      candidates = results.select { |label| human_label?(label) } if candidates.empty?
      candidates.min_by { |label| Array(label.dig('openfda', 'substance_name')).size } || results.first
    end

    def human_label?(label)
      Array(label.dig('openfda', 'product_type')).any? { |type| type.start_with?('HUMAN ') }
    end

    def substance_matches?(label, term)
      normalized_term = MedicationReviewTermNormalizer.label(term)
      Array(label.dig('openfda', 'substance_name')).any? do |substance|
        MedicationReviewTermNormalizer.label(substance).include?(normalized_term)
      end
    end

    def request_json(uri)
      response = Net::HTTP.start(
        uri.host, uri.port, use_ssl: uri.scheme == 'https', open_timeout: OPEN_TIMEOUT, read_timeout: READ_TIMEOUT
      ) { |http| http.get(uri.request_uri) }
      raise NotFound if response.code == '404'
      raise "openFDA label request failed with HTTP #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body)
    end

    def concurrent_responses(entries)
      queue = Queue.new
      entries.each_with_index { |entry, index| queue << [index, entry] }
      responses = Array.new(entries.size)
      errors = Queue.new
      workers = Array.new([WORKER_COUNT, entries.size].min) { response_worker(queue, responses, errors) }
      workers.each(&:join)
      raise errors.pop unless errors.empty?

      responses
    end

    def response_worker(queue, responses, errors)
      Thread.new do
        loop do
          index, entry = queue.pop(true)
          responses[index] = response_for(entry)
        rescue ThreadError
          break
        rescue StandardError => e
          errors << e
          break
        end
      end
    end

    def searches_for(term, interaction_targets)
      target_searches = interaction_targets.map { |target| %(drug_interactions:"#{target}") }
      %w[generic_name substance_name].flat_map do |field|
        [
          ["openfda.#{field}.exact:\"#{term.upcase}\"", '_exists_:drug_interactions', *target_searches].join(' AND '),
          ["openfda.#{field}:\"#{term}\"", '_exists_:drug_interactions', *target_searches].join(' AND ')
        ]
      end
    end

    def request_uri(search:, limit:, sort: nil)
      uri = URI(ENDPOINT)
      query = { search: search, limit: limit }
      query[:sort] = sort if sort
      uri.query = URI.encode_www_form(query)
      uri
    end
  end
end
