# Task 4 report: bounded external-I/O concurrency

## Outcome

OpenFDA and NLM batch clients now share a bounded ordered thread mapper. Both clients accept an optional `worker_count:`
that defaults to eight and rejects non-positive values. Worker creation is capped to the batch size, result ordering follows
input ordering, empty batches do not start workers, and the first recorded failure stops further dequeueing. In-flight work
is joined before the first error is propagated.

OpenFDA empty batches return the existing response shape with an empty metadata object and results array:
`{ 'meta' => {}, 'results' => [] }`. NLM empty batches return `[]`.

The existing Net::HTTP open timeout of 5 seconds and read timeout of 20 seconds remain unchanged in both clients.

## Red-green-refactor evidence

The focused baseline passed with 3 examples and 0 failures.

The RED run added public-operation coverage before production changes and failed with 11 failures. The primary failure was
`ArgumentError: wrong number of arguments (given 1, expected 0)` for `new(worker_count:)`. The OpenFDA empty-batch contract
also failed at `responses.first.fetch('meta')`.

After the shared mapper and client wiring were implemented, the focused run passed with 16 examples and 0 failures:

```text
task test TEST_FILE='spec/services/open_fda/drug_label_client_spec.rb spec/services/nlm/rx_class_client_spec.rb'
16 examples, 0 failures
```

Coverage includes default and custom worker bounds, ordering, non-positive worker rejection, empty batches, no dequeue after
failure, first-error propagation, joining in-flight requests, and unchanged HTTP timeout options. All request behavior is
stubbed or supplied by deterministic in-process test clients; no real network requests run.

## Serial/parallel delay comparison

The contract runs eight deterministic 20 ms delayed NLM requests through the public `entries_for` operation with one worker
and then four workers, asserting identical results and requiring parallel duration to be less than 80% of serial duration.

```text
worker_count: 1  0.1762s
worker_count: 4  0.0435s
improvement         75.3%
```

The parallel path exceeds the required 20% improvement gate and is retained.

## Quality checks

`task test:preflight` passed with 14 examples and 0 failures.

`task rubocop` inspected 1,650 files. Task 4 offenses reported during refactoring were fixed. The repository-wide command
remains blocked by two unrelated pre-existing offenses in `spec/config/audition_toolchain_spec.rb`:

- `RSpec/DescribeClass` at line 5
- `RSpec/ExampleLength` at line 32
