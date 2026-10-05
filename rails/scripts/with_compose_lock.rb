#!/usr/bin/env ruby

# frozen_string_literal: true

require 'digest'
require 'tmpdir'

lock_key = ARGV.shift
abort 'Usage: with_compose_lock.rb LOCK_KEY COMMAND [ARGUMENTS...]' if lock_key.nil? || ARGV.empty?

lock_path = File.join(Dir.tmpdir, "medtracker-compose-#{Digest::SHA256.hexdigest(lock_key)}.lock")

File.open(lock_path, File::RDWR | File::CREAT, 0o600) do |lock|
  lock.flock(File::LOCK_EX)
  command = ARGV
  pid = Process.spawn(*command)
  _finished_pid, status = Process.wait2(pid)
  exit(status.exitstatus || 128 + status.termsig)
end
