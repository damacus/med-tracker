# Audit Trail Documentation

## Overview

MedTracker uses [PaperTrail](https://github.com/paper-trail-gem/paper_trail) to maintain a complete audit trail of all changes to critical data models. This ensures compliance with UK healthcare regulations and provides accountability for all system actions.

## Audited Models

The following models have audit trail enabled:

- **User**: Track user account changes (excluding password fields for security)
- **Person**: Track patient/carer demographic changes
- **CarerRelationship**: Track carer assignments and removals
- **MedicationTake**: Track all medication doses (critical for patient safety)

## Accessing Audit Logs

- **URL**: `/admin/audit_logs`
- **Access**: Administrators only (enforced by `AuditLogPolicy`)
- **Features**:
  - Filter by record type (User, Person, etc.)
  - Filter by event type (create, update, destroy)
  - View complete change history with timestamps
  - See who made changes and from which IP address
  - View previous state of records before changes
  - Read-only interface (no editing or deletion of audit logs)

## Technical Details

### Database Schema

Audit logs are stored in the `versions` table with the following key fields:

| Column | Type | Description |
|--------|------|-------------|
| `item_type` | string | Model class name (e.g., "User", "Person") |
| `item_id` | bigint | ID of the record that changed |
| `event` | string | Type of change: "create", "update", or "destroy" |
| `whodunnit` | string | User ID who made the change |
| `ip` | string | IP address of the request |
| `object` | text | YAML snapshot of record state before change |
| `created_at` | datetime | When the change occurred |

### Configuration

See `config/initializers/paper_trail.rb` for:

- Permitted YAML classes for safe deserialization
- Default tracking options (create, update, destroy)
- Global PaperTrail settings

### User Tracking

The `ApplicationController` sets the audit context:

```ruby
def user_for_paper_trail
  current_user&.id
end

def info_for_paper_trail
  { ip: request.remote_ip }
end
```

This ensures every change is attributed to a specific user and IP address.

### Adding Audit Trail to New Models

To enable audit tracking on a new model:

```ruby
class YourModel < ApplicationRecord
  has_paper_trail
  
  # Optional: exclude sensitive fields
  # has_paper_trail ignore: %i[password_field secret_field]
  
  # Optional: track only specific events
  # has_paper_trail on: %i[create destroy]
end
```

## Compliance & Regulatory Requirements

This audit trail supports compliance with:

- **UK GDPR Article 32**: Security of processing - maintaining records of data access and changes
- **DCB0129**: Clinical Risk Management - tracking medication and patient data changes
- **DCB0160**: Clinical Safety - audit trail for safety-critical operations
- **NHS Data Security and Protection Toolkit**: Evidence of access controls and audit logging

## Data Retention

Audit logs are retained **indefinitely** for regulatory compliance and legal requirements. The `versions` table should be monitored for growth and may require archival strategies for long-term deployments.

## Performance Considerations

- Audit logs are written synchronously with each database change
- The `versions` table will grow continuously - plan for database growth
- Queries are optimized with indexes on `item_type`, `item_id`, and `created_at`
- Consider archiving old audit logs (>7 years) to separate storage if needed

## Security

- **Read-only**: Audit logs cannot be edited or deleted through the UI
- **Administrator access only**: Only users with `administrator` role can view audit logs
- **IP tracking**: All changes include the originating IP address
- **Password exclusion**: Password fields are never stored in audit logs
- **Rate limiting**: To prevent abuse and DoS attacks on sensitive audit data:
  - 100 requests per minute per IP address
  - 200 requests per minute per authenticated user
  - Violations are logged for monitoring

## Testing

Audit trail functionality is tested in:

- `spec/models/*_spec.rb` - Model-level versioning tests
- `spec/policies/audit_log_policy_spec.rb` - Authorization tests
- `spec/components/admin/audit_logs/*_spec.rb` - UI component tests

## Future Enhancements

Potential improvements for consideration:

1. **Export functionality**: Allow administrators to export audit logs as CSV/JSON
2. **Advanced search**: Full-text search across change data
3. **Diff view**: Visual comparison of before/after states
4. **Alerts**: Notify administrators of suspicious patterns
5. **Retention policies**: Automated archival of old logs
