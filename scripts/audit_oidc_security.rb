#!/usr/bin/env ruby
# frozen_string_literal: true

require 'pathname'

class OidcSecurityAudit
  CHECKS = %i[
    check_no_hardcoded_secrets
    check_gitignore_patterns
    check_credentials_encrypted
    check_master_key_not_tracked
    check_env_files_not_tracked
  ].freeze

  def initialize(root: Pathname.new(File.expand_path('..', __dir__)))
    @root = root
    @passed = 0
    @failed = 0
    @warnings = 0
  end

  def run # rubocop:disable Naming/PredicateMethod
    puts "OIDC Security Audit — #{@root}"
    puts '=' * 60

    CHECKS.each { |check| send(check) }

    puts
    puts '=' * 60
    puts "Results: #{@passed} passed, #{@failed} failed, #{@warnings} warnings"
    @failed.zero?
  end

  private

  def record_pass(message)
    @passed += 1
    puts "  PASS: #{message}"
  end

  def record_fail(message)
    @failed += 1
    puts "  FAIL: #{message}"
  end

  def record_warn(message)
    @warnings += 1
    puts "  WARN: #{message}"
  end

  def check_no_hardcoded_secrets
    puts "\nChecking for hardcoded secrets..."
    source_files = Dir.glob(@root.join('{app,config,lib}/**/*.{rb,yml,yaml,erb}'))
    violations = source_files.select do |file|
      next false if file.end_with?('.yml.enc')
      next false if file.include?('audit_oidc_security')

      content = File.read(file)
      content.match?(/OIDC_CLIENT_SECRET\s*=\s*['"][^'"]+['"]/)
    end

    if violations.empty?
      record_pass 'No hardcoded OIDC secrets found in source files'
    else
      violations.each { |f| record_fail "Hardcoded secret in: #{f}" }
    end
  end

  def check_gitignore_patterns
    puts "\n🔍 Checking .gitignore patterns..."
    gitignore = @root.join('.gitignore')
    return record_warn('.gitignore not found') unless gitignore.exist?

    content = gitignore.read
    if content.include?('master.key') || content.include?('*.key')
      record_pass '.gitignore excludes master key files'
    else
      record_warn '.gitignore does not explicitly exclude master.key'
    end

    if content.include?('.env')
      record_pass '.gitignore excludes .env files'
    else
      record_warn '.gitignore does not explicitly exclude .env files'
    end
  end

  def check_credentials_encrypted
    puts "\n🔍 Checking credentials encryption..."
    enc_file = @root.join('config/credentials.yml.enc')
    plain_file = @root.join('config/credentials.yml')

    if plain_file.exist?
      record_fail 'Plaintext credentials.yml exists - must be encrypted'
    else
      record_pass 'No plaintext credentials.yml found'
    end

    if enc_file.exist?
      record_pass 'Encrypted credentials file exists (credentials.yml.enc)'
    else
      record_warn 'No encrypted credentials file found (optional if using env vars only)'
    end
  end

  def check_master_key_not_tracked
    puts "\n🔍 Checking master key is not tracked..."
    tracked = `git -C #{@root} ls-files config/master.key 2>/dev/null`.strip
    if tracked.empty?
      record_pass 'config/master.key is not tracked in git'
    else
      record_fail 'config/master.key IS tracked in git - remove immediately'
    end
  end

  def check_env_files_not_tracked
    puts "\n🔍 Checking .env files are not tracked..."
    tracked_env = `git -C #{@root} ls-files .env .env.local .env.production 2>/dev/null`.strip

    if tracked_env.empty?
      record_pass 'No .env files tracked in git'
    else
      tracked_env.split("\n").each { |f| record_fail "#{f} IS tracked in git" }
    end
  end
end

if __FILE__ == $PROGRAM_NAME
  audit = OidcSecurityAudit.new
  exit(audit.run ? 0 : 1)
end
