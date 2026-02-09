# frozen_string_literal: true

require 'rails_helper'
require 'otel/span_sanitizer'

RSpec.describe Otel::SpanSanitizer do
  subject(:sanitizer) { described_class.new }

  describe '#sanitize_value' do
    it 'redacts email addresses' do
      expect(sanitizer.sanitize_value('user@example.com')).to eq('[EMAIL REDACTED]')
    end

    it 'redacts emails embedded in longer strings' do
      result = sanitizer.sanitize_value('Contact john.doe@example.com for details')
      expect(result).to eq('Contact [EMAIL REDACTED] for details')
    end

    it 'redacts multiple emails in one string' do
      result = sanitizer.sanitize_value('from alice@test.com to bob@test.com')
      expect(result).not_to include('alice@test.com')
      expect(result).not_to include('bob@test.com')
    end

    it 'redacts date-of-birth patterns (YYYY-MM-DD)' do
      expect(sanitizer.sanitize_value('1990-05-15')).to eq('[DATE REDACTED]')
    end

    it 'redacts date-of-birth patterns (DD/MM/YYYY)' do
      expect(sanitizer.sanitize_value('15/05/1990')).to eq('[DATE REDACTED]')
    end

    it 'redacts IP addresses' do
      expect(sanitizer.sanitize_value('192.168.1.100')).to eq('[IP REDACTED]')
    end

    it 'redacts IP addresses embedded in strings' do
      result = sanitizer.sanitize_value('Client IP: 10.0.0.1 connected')
      expect(result).to eq('Client IP: [IP REDACTED] connected')
    end

    it 'preserves non-sensitive strings' do
      expect(sanitizer.sanitize_value('medication_take.create')).to eq('medication_take.create')
    end

    it 'preserves numeric IDs' do
      expect(sanitizer.sanitize_value('12345')).to eq('12345')
    end

    it 'preserves ISO 8601 timestamps with time component' do
      expect(sanitizer.sanitize_value('2025-01-15T10:30:00Z')).to eq('2025-01-15T10:30:00Z')
    end

    it 'returns non-string values unchanged' do
      expect(sanitizer.sanitize_value(42)).to eq(42)
      expect(sanitizer.sanitize_value(true)).to be(true)
      expect(sanitizer.sanitize_value(nil)).to be_nil
    end
  end

  describe '#sanitize_attributes' do
    it 'redacts values for sensitive attribute keys' do
      attrs = {
        'http.request.header.authorization' => 'Bearer secret-token',
        'http.request.header.cookie' => 'session=abc123',
        'model.name' => 'MedicationTake'
      }

      result = sanitizer.sanitize_attributes(attrs)

      expect(result['http.request.header.authorization']).to eq('[REDACTED]')
      expect(result['http.request.header.cookie']).to eq('[REDACTED]')
      expect(result['model.name']).to eq('MedicationTake')
    end

    it 'redacts PII patterns in attribute values even for non-sensitive keys' do
      attrs = {
        'event.detail' => 'born on 1990-05-15 in London',
        'log.message' => 'Contact user@example.com now',
        'model.id' => '42'
      }

      result = sanitizer.sanitize_attributes(attrs)

      expect(result['event.detail']).to eq('born on [DATE REDACTED] in London')
      expect(result['log.message']).to eq('Contact [EMAIL REDACTED] now')
      expect(result['model.id']).to eq('42')
    end

    it 'redacts values for keys containing name, email, or password' do
      attrs = {
        'person.name' => 'John Doe',
        'user.email_address' => 'john@example.com',
        'db.password' => 'secret'
      }

      result = sanitizer.sanitize_attributes(attrs)

      expect(result['person.name']).to eq('[REDACTED]')
      expect(result['user.email_address']).to eq('[REDACTED]')
      expect(result['db.password']).to eq('[REDACTED]')
    end

    it 'does not modify the original hash' do
      attrs = { 'user.email' => 'test@example.com' }
      sanitizer.sanitize_attributes(attrs)
      expect(attrs['user.email']).to eq('test@example.com')
    end
  end

  describe '.sensitive_key?' do
    it 'identifies authorization headers as sensitive' do
      expect(described_class.sensitive_key?('http.request.header.authorization')).to be(true)
    end

    it 'identifies cookie headers as sensitive' do
      expect(described_class.sensitive_key?('http.request.header.cookie')).to be(true)
    end

    it 'identifies name fields as sensitive' do
      expect(described_class.sensitive_key?('person.name')).to be(true)
    end

    it 'identifies email fields as sensitive' do
      expect(described_class.sensitive_key?('user.email_address')).to be(true)
    end

    it 'identifies password fields as sensitive' do
      expect(described_class.sensitive_key?('db.password')).to be(true)
      expect(described_class.sensitive_key?('password_digest')).to be(true)
    end

    it 'identifies date_of_birth fields as sensitive' do
      expect(described_class.sensitive_key?('person.date_of_birth')).to be(true)
    end

    it 'does not flag model.name as sensitive' do
      expect(described_class.sensitive_key?('model.name')).to be(false)
    end

    it 'does not flag model.id as sensitive' do
      expect(described_class.sensitive_key?('model.id')).to be(false)
    end

    it 'does not flag model.operation as sensitive' do
      expect(described_class.sensitive_key?('model.operation')).to be(false)
    end
  end
end
