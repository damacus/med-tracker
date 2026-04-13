# 5. Use PostgreSQL 18 as the Database

Date: 2026-04-13

## Status

Accepted

## Context

MedTracker needs a reliable relational database that supports advanced features, high concurrency, and robust data integrity for sensitive health data.

## Decision

Use PostgreSQL 18 as the project's standard database for all environments (development, test, production).

## Consequences

- Full access to PostgreSQL 18 performance improvements and features.
- Consistent configuration across all environments using Docker.
- Compliance with the latest industry database standards.
