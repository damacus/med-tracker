# frozen_string_literal: true

module ExternalIo
  class OrderedThreadMap
    class State
      attr_reader :results

      def initialize(entries)
        @pending = entries.each_with_index.map { |entry, index| [index, entry] }
        @results = Array.new(entries.size)
        @mutex = Mutex.new
        @error = nil
      end

      def next_entry
        @mutex.synchronize { @error ? nil : @pending.shift }
      end

      def error=(error)
        @mutex.synchronize { @error ||= error }
      end

      def error
        @mutex.synchronize { @error }
      end
    end

    def initialize(worker_count:)
      @worker_count = Integer(worker_count)
      raise ArgumentError, 'worker_count must be positive' unless @worker_count.positive?
    end

    def call(entries, &operation)
      return [] if entries.empty?

      state = State.new(entries)
      workers = Array.new(worker_total(entries)) { build_worker(state, operation) }
      workers.each(&:join)
      raise state.error if state.error

      state.results
    end

    private

    def worker_total(entries)
      [@worker_count, entries.size].min
    end

    def build_worker(state, operation)
      Thread.new do
        loop do
          indexed_entry = state.next_entry
          break unless indexed_entry

          index, entry = indexed_entry
          state.results[index] = operation.call(entry)
        rescue StandardError => e
          state.error = e
          break
        end
      end
    end
  end
end
