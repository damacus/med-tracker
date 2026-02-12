# frozen_string_literal: true

require 'test_helper'

class PrescriptionsControllerTest < ActionDispatch::IntegrationTest
  def setup
    @person = people(:john)
    @medicine = medicines(:paracetamol)
    @dosage = dosages(:paracetamol_light)
    @prescription = prescriptions(:john_paracetamol)
    sign_in(users(:john))
  end

  test 'should get new prescription form via turbo stream' do
    get new_person_prescription_path(@person), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
    assert_response :success
    assert_match(/turbo-stream/, @response.content_type)
  end

  test 'should create prescription' do
    assert_difference('Prescription.count') do
      post person_prescriptions_path(@person), params: {
        prescription: {
          medicine_id: @medicine.id,
          dosage_id: @dosage.id,
          frequency: 'Every 6 hours',
          start_date: Date.current,
          end_date: Date.current + 7.days,
          notes: 'Test prescription',
          max_daily_doses: 4,
          min_hours_between_doses: 4,
          dose_cycle: 'daily'
        }
      }
    end

    assert_redirected_to person_path(@person)
    assert_equal 'Prescription was successfully created.', flash[:notice]
  end

  test 'should not create prescription with invalid params' do
    assert_no_difference('Prescription.count') do
      post person_prescriptions_path(@person), params: {
        prescription: {
          medicine_id: nil,
          dosage_id: nil,
          frequency: nil,
          start_date: nil
        }
      }, headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
    end

    assert_response :unprocessable_entity
    assert_match(/turbo-stream/, @response.content_type)
  end

  test 'should update prescription' do
    patch person_prescription_path(@person, @prescription), params: {
      prescription: {
        dosage_id: @dosage.id,
        notes: 'Updated notes'
      }
    }

    assert_redirected_to person_path(@person)
    @prescription.reload
    assert_equal @dosage.id, @prescription.dosage_id
    assert_equal 'Updated notes', @prescription.notes
  end

  test 'should destroy prescription' do
    assert_difference('Prescription.count', -1) do
      delete person_prescription_path(@person, @prescription)
    end

    assert_redirected_to person_path(@person)
    assert_equal 'Prescription was successfully deleted.', flash[:notice]
  end
end
