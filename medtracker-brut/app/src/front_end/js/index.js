import { BrutCustomElements } from "brut-js"

import Example from "./Example"
document.addEventListener("DOMContentLoaded", () => {
  Example.define()
  BrutCustomElements.define() // Define all of BrutJS's autnomous custom elements
})
