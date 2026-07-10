# frozen_string_literal: true

require 'net/http'

module Nlm
  class RxClassClient
    ENDPOINT = 'https://rxnav.nlm.nih.gov/REST/rxclass/class/byDrugName.json'
    INCLUDED_CLASS_TYPES = %w[CHEM EPC].freeze
    OPEN_TIMEOUT = 5
    READ_TIMEOUT = 20
    WORKER_COUNT = 8

    def entries_for(terms)
      responses_for(terms).zip(terms).map { |response, term| entry_for(response, term) }
    end

    private

    def responses_for(terms)
      queue = Queue.new
      terms.each_with_index { |term, index| queue << [index, term] }
      responses = Array.new(terms.size)
      errors = Queue.new
      workers = Array.new([WORKER_COUNT, terms.size].min) { response_worker(queue, responses, errors) }
      workers.each(&:join)
      raise errors.pop unless errors.empty?

      responses
    end

    def response_worker(queue, responses, errors)
      Thread.new do
        loop do
          index, term = queue.pop(true)
          responses[index] = request_json(request_uri(term))
        rescue ThreadError
          break
        rescue StandardError => e
          errors << e
          break
        end
      end
    end

    def request_uri(term)
      uri = URI(ENDPOINT)
      uri.query = URI.encode_www_form(drugName: term, relaSource: 'DailyMed')
      uri
    end

    def request_json(uri)
      response = Net::HTTP.start(
        uri.host, uri.port, use_ssl: true, open_timeout: OPEN_TIMEOUT, read_timeout: READ_TIMEOUT
      ) { |http| http.get(uri.request_uri) }
      raise "RxClass request failed with HTTP #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body)
    end

    def entry_for(response, term)
      drug_info = Array(response.dig('rxclassDrugInfoList', 'rxclassDrugInfo'))
      ingredient = drug_info.filter_map { |info| info['minConcept'] }.find { |concept| concept['tty'].in?(%w[IN PIN]) }
      {
        'selection_term' => term,
        'rxcui' => ingredient&.fetch('rxcui', nil),
        'ingredient_name' => ingredient&.fetch('name', nil) || term,
        'classes' => class_entries(drug_info)
      }
    end

    def class_entries(drug_info)
      drug_info.filter_map do |info|
        class_item = info['rxclassMinConceptItem']
        next unless class_item && class_item['classType'].in?(INCLUDED_CLASS_TYPES)

        {
          'id' => class_item.fetch('classId'),
          'name' => class_item.fetch('className'),
          'type' => class_item.fetch('classType')
        }
      end.uniq
    end
  end
end
