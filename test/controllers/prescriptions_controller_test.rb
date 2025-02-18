require "test_helper"

class PrescriptionsControllerTest < ActionDispatch::IntegrationTest
  def setup
    @person = people(:adult_john)
    @medicine = medicines(:paracetamol)
    @prescription = prescriptions(:john_paracetamol)
  end

  test "should get new prescription form via turbo stream" do
    get new_person_prescription_path(@person), headers: { "Accept" => "text/vnd.turbo-stream.html" }
    assert_response :success
    assert_match /turbo-stream/, @response.content_type
  end

  test "should create prescription" do
    assert_difference("Prescription.count") do
      post person_prescriptions_path(@person), params: {
        prescription: {
          medicine_id: @medicine.id,
          dosage: 1.0,
          frequency: "Every 6 hours",
          start_date: Date.current,
          end_date: Date.current + 7.days,
          notes: "Test prescription"
        }
      }
    end

    assert_redirected_to person_path(@person)
    assert_equal "Prescription was successfully created.", flash[:notice]
  end

  test "should not create prescription with invalid params" do
    assert_no_difference("Prescription.count") do
      post person_prescriptions_path(@person), params: {
        prescription: {
          medicine_id: nil,
          dosage: nil,
          frequency: nil,
          start_date: nil
        }
      }, headers: { "Accept" => "text/vnd.turbo-stream.html" }
    end

    assert_response :unprocessable_entity
    assert_match /turbo-stream/, @response.content_type
  end

  test "should update prescription" do
    patch person_prescription_path(@person, @prescription), params: {
      prescription: {
        dosage: 2.0,
        notes: "Updated notes"
      }
    }

    assert_redirected_to person_path(@person)
    @prescription.reload
    assert_equal 2.0, @prescription.dosage.to_f
    assert_equal "Updated notes", @prescription.notes
  end

  test "should not update prescription with invalid params" do
    patch person_prescription_path(@person, @prescription), params: {
      prescription: {
        dosage: nil,
        frequency: nil
      }
    }, headers: { "Accept" => "text/vnd.turbo-stream.html" }

    assert_response :unprocessable_entity
    assert_match /turbo-stream/, @response.content_type
  end

  test "should destroy prescription" do
    assert_difference("Prescription.count", -1) do
      delete person_prescription_path(@person, @prescription)
    end

    assert_redirected_to person_path(@person)
    assert_equal "Prescription was successfully deleted.", flash[:notice]
  end
end
