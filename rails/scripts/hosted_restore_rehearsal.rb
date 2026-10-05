#!/usr/bin/env ruby

# frozen_string_literal: true

require_relative '../lib/hosted_restore/rehearsal'

begin
  exit HostedRestore::Rehearsal.new.call
rescue HostedRestore::Input::Invalid => e
  warn JSON.generate(outcome: 'failed', failure_code: e.message)
  exit(2)
rescue HostedRestore::EvidenceWriter::AlreadyExists => e
  warn JSON.generate(outcome: 'failed', failure_code: e.message)
  exit(2)
end
