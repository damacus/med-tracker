# Ractor decision

MedTracker does not currently use Ractor for production work.

The measured dashboard hotspot is an interactive Rails request coupled to
Phlex components, policy-scoped Active Record collections, SQL results, and
request context. It is not a primitive-data CPU batch. The only measured batch
speedup is external API fan-out, where four bounded threads were 75.3% faster
than one worker because requests spend their time waiting on external I/O.

Audition `0.2.1` on Ruby `4.0.6` reported no static findings for the dashboard
component target:

```fish
task audition:target TARGET=app/components/dashboard/index_view.rb
```

The configured bounded dynamic probe could not boot Rails because its tools
container had no PostgreSQL socket:

```fish
task audition:dynamic
```

This was recorded as inconclusive rather than treated as proof of readiness.
No fixes were applied.

The operational boundaries remain:

- bounded threads for independent external-I/O requests;
- background jobs for durable application batches and tenant reconstruction;
- worker processes for Rails and database isolation.

Ractor should be reconsidered only when a representative profile identifies a
CPU-bound batch that accepts and returns shareable primitive data. Adoption
then requires a successful target and dynamic Audition assessment plus a
repeatable improvement of at least 20% over the best serial or process result,
including worker startup and data-transfer overhead.
