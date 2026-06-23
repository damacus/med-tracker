import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["field"]
  static values = { roles: Array }

  connect() {
    this.sync()
  }

  sync() {
    if (!this.hasFieldTarget) return

    const enabled = this.rolesValue.includes(this.selectedRole())
    this.fieldTarget.hidden = !enabled
    this.fieldTarget.querySelectorAll("input").forEach((input) => {
      input.disabled = !enabled
    })
  }

  selectedRole() {
    const checkedRelationship = this.element.querySelector("input[name='user[dependent_relationship_type]']:checked")
    if (checkedRelationship) return checkedRelationship.value

    const checkedRole = this.element.querySelector("input[name='user[role]']:checked")
    if (checkedRole) return checkedRole.value

    const invitationRelationshipType = this.element.querySelector("select[name='invitation[relationship_type]']")?.value
    if (invitationRelationshipType) return invitationRelationshipType

    return this.element.querySelector("select[name='invitation[role]']")?.value
  }
}
