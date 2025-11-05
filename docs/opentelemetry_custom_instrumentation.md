# Custom OpenTelemetry Instrumentation Examples

This document provides examples of how to add custom instrumentation to your application code using OpenTelemetry. While automatic instrumentation covers most Rails components, custom instrumentation can provide more detailed insights into your business logic.

## Basic Custom Spans

### Example 1: Adding Spans to Controller Actions

```ruby
class MedicationTakesController < ApplicationController
  def create
    tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
    
    tracer.in_span('medication_take.create') do |span|
      @medication_take = @prescription.medication_takes.build(medication_take_params)
      
      # Add attributes to the span for filtering and analysis
      span.set_attribute('prescription.id', @prescription.id)
      span.set_attribute('medicine.name', @prescription.medicine.name)
      span.set_attribute('person.id', @prescription.person_id)
      
      authorize @medication_take
      
      if @medication_take.save
        span.add_event('medication_take.recorded', attributes: {
          'take.id' => @medication_take.id,
          'take.taken_at' => @medication_take.taken_at.to_s
        })
        
        respond_to do |format|
          format.html { redirect_back(fallback_location: root_path, notice: t('take_medicines.success')) }
          format.json { render json: { success: true, message: 'Medication taken successfully recorded.' } }
        end
      else
        span.add_event('medication_take.failed', attributes: {
          'errors' => @medication_take.errors.full_messages.join(', ')
        })
        
        respond_to do |format|
          format.html { redirect_back(fallback_location: root_path, alert: t('take_medicines.failure')) }
          format.json do
            render json: { success: false, errors: @medication_take.errors.full_messages }, status: :unprocessable_entity
          end
        end
      end
    end
  end
end
```

### Example 2: Tracing Business Logic in Models

```ruby
class Prescription < ApplicationRecord
  def check_dosage_timing
    tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
    
    tracer.in_span('prescription.check_dosage_timing') do |span|
      span.set_attribute('prescription.id', id)
      span.set_attribute('prescription.max_daily_doses', max_daily_doses)
      span.set_attribute('prescription.min_hours_between_doses', min_hours_between_doses)
      
      # Your business logic here
      recent_takes = medication_takes.where('taken_at >= ?', 24.hours.ago)
      span.set_attribute('recent_takes.count', recent_takes.count)
      
      if recent_takes.count >= max_daily_doses
        span.add_event('dosage.limit_reached')
        return false
      end
      
      # Check minimum hours between doses
      if recent_takes.any?
        last_take = recent_takes.maximum(:taken_at)
        hours_since_last = ((Time.current - last_take) / 1.hour).round(2)
        span.set_attribute('hours_since_last_dose', hours_since_last)
        
        if hours_since_last < min_hours_between_doses
          span.add_event('dosage.too_soon', attributes: {
            'hours_since_last' => hours_since_last,
            'required_hours' => min_hours_between_doses
          })
          return false
        end
      end
      
      span.add_event('dosage.check_passed')
      true
    end
  end
end
```

### Example 3: Tracing Service Objects

```ruby
class MedicineStockChecker
  def self.check_and_alert(medicine_id)
    tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
    
    tracer.in_span('medicine.stock_check') do |span|
      medicine = Medicine.find(medicine_id)
      
      span.set_attribute('medicine.id', medicine.id)
      span.set_attribute('medicine.name', medicine.name)
      span.set_attribute('medicine.current_stock', medicine.current_stock)
      span.set_attribute('medicine.reorder_threshold', medicine.reorder_threshold)
      
      if medicine.current_stock <= medicine.reorder_threshold
        span.add_event('medicine.reorder_required', attributes: {
          'shortfall' => medicine.reorder_threshold - medicine.current_stock
        })
        
        # Send notification
        NotificationService.send_reorder_alert(medicine)
      end
    end
  end
end
```

## Advanced Patterns

### Exception Handling with Spans

```ruby
def process_prescription
  tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
  
  tracer.in_span('prescription.process') do |span|
    begin
      # Your logic here
      prescription.save!
      span.add_event('prescription.saved')
    rescue ActiveRecord::RecordInvalid => e
      span.record_exception(e)
      span.status = OpenTelemetry::Trace::Status.error("Validation failed: #{e.message}")
      raise
    end
  end
end
```

### Nested Spans

```ruby
def create_prescription_with_stock_update
  tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
  
  tracer.in_span('prescription.create_with_stock') do |parent_span|
    parent_span.set_attribute('person.id', person_id)
    
    # Child span for prescription creation
    tracer.in_span('prescription.create') do |span|
      @prescription = Prescription.create!(prescription_params)
      span.set_attribute('prescription.id', @prescription.id)
    end
    
    # Child span for stock update
    tracer.in_span('medicine.update_stock') do |span|
      medicine = @prescription.medicine
      medicine.update_stock!
      span.set_attribute('medicine.new_stock', medicine.current_stock)
    end
    
    parent_span.add_event('prescription_and_stock.completed')
  end
end
```

### Background Job Instrumentation

```ruby
class MedicationReminderJob < ApplicationJob
  def perform(prescription_id)
    tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
    
    tracer.in_span('job.medication_reminder') do |span|
      span.set_attribute('job.prescription_id', prescription_id)
      span.set_attribute('job.queue', queue_name)
      
      prescription = Prescription.find(prescription_id)
      span.set_attribute('prescription.person_id', prescription.person_id)
      
      # Send reminder logic
      result = ReminderService.send_reminder(prescription)
      
      span.add_event('reminder.sent', attributes: {
        'reminder.method' => result[:method],
        'reminder.success' => result[:success]
      })
    end
  end
end
```

## Best Practices

### 1. Naming Conventions

Use dot notation for span names:
- `resource.action` - e.g., `prescription.create`, `medicine.update`
- `service.operation` - e.g., `notification.send`, `stock.check`

### 2. Attribute Guidelines

- Use lowercase with underscores: `medicine.id`, `person.name`
- Include identifiers: IDs, names, types
- Avoid sensitive data: passwords, API keys, PII
- Keep values simple: strings, numbers, booleans

### 3. Event Usage

Add events for important moments:
```ruby
span.add_event('dosage.limit_reached')
span.add_event('prescription.created', attributes: { 'id' => prescription.id })
span.add_event('validation.failed', attributes: { 'errors' => errors.join(', ') })
```

### 4. Error Handling

Always record exceptions:
```ruby
rescue StandardError => e
  span.record_exception(e)
  span.status = OpenTelemetry::Trace::Status.error(e.message)
  raise
end
```

### 5. Performance Considerations

- Don't create too many spans - focus on business-critical operations
- Avoid adding large strings as attributes
- Use sampling in production for high-volume operations

## Viewing Your Traces

Once you've added custom instrumentation:

1. Start your application with OpenTelemetry enabled
2. Perform the instrumented actions
3. View traces in your backend (e.g., Jaeger at http://localhost:16686)
4. Look for your custom span names and attributes
5. Use attributes to filter and analyze specific scenarios

## Common Use Cases for MedTracker

### Track Medication Takes
- Span: `medication_take.create`
- Attributes: `prescription.id`, `medicine.name`, `person.id`
- Events: `take.recorded`, `validation.failed`

### Monitor Prescription Creation
- Span: `prescription.create`
- Attributes: `medicine.id`, `dosage.amount`, `frequency`
- Events: `prescription.created`, `prescription.failed`

### Track Stock Levels
- Span: `medicine.stock_check`
- Attributes: `medicine.id`, `current_stock`, `reorder_threshold`
- Events: `reorder.required`, `stock.sufficient`

### Authorization Checks
- Span: `authorization.check`
- Attributes: `user.role`, `resource.type`, `action`
- Events: `authorization.granted`, `authorization.denied`

## Further Reading

- [OpenTelemetry Ruby API Documentation](https://www.rubydoc.info/gems/opentelemetry-api)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [Ruby Instrumentation Examples](https://github.com/open-telemetry/opentelemetry-ruby-contrib/tree/main/examples)
