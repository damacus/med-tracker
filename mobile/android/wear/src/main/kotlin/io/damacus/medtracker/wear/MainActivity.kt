package io.damacus.medtracker.wear

import android.app.Activity
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.AbsoluteSizeSpan
import android.text.style.ForegroundColorSpan
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.damacus.medtracker.wear.protocol.CompanionSchedule
import io.damacus.medtracker.wear.protocol.MedicationData
import io.damacus.medtracker.wear.protocol.PersonData
import io.damacus.medtracker.wear.protocol.TakenDoseRecord
import java.util.ArrayDeque
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

sealed interface Screen {
    data class Status(val state: ConnectionState) : Screen
    data object Home : Screen
    data object PersonChooser : Screen
    data class MedicationChooser(val person: PersonData) : Screen
    data class TakeNow(val person: PersonData, val medication: MedicationData) : Screen
    data class DoseRecorded(val medication: MedicationData, val isReachable: Boolean) : Screen
}

class MainActivity : Activity() {
    private lateinit var dataLayer: WearDataLayer
    private lateinit var scrollView: ScrollView
    private lateinit var container: LinearLayout
    private var observation: Job? = null
    private var dismissJob: Job? = null

    private var currentScreen: Screen = Screen.Status(ConnectionState.WAITING_FOR_STATUS)
    private val screenStack = ArrayDeque<Screen>()

    private var latestSnapshot: CompanionSnapshot? = null

    private val defaultPeople = listOf(
        PersonData("p-self", "Self (John)", 0),
        PersonData("p-maya", "Maya (Child)", 1),
        PersonData("p-alex", "Alex (Dependent)", 2)
    )

    private val defaultMedications = listOf(
        MedicationData("m-amox", "p-maya", "Amoxicillin", "250mg • 1 capsule", "due_now", "10:00 AM"),
        MedicationData("m-salb", "p-maya", "Salbutamol", "2 puffs", "as_needed", null),
        MedicationData("m-ator", "p-self", "Atorvastatin", "20mg • 1 tablet", "scheduled", "8:00 PM"),
        MedicationData("m-parac", "p-alex", "Paracetamol", "500mg • 2 tablets", "as_needed", null)
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        dataLayer = WearDataLayer(applicationContext)

        container = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val padH = dp(16)
            val padV = dp(32)
            setPadding(padH, padV, padH, padV)
            gravity = Gravity.CENTER_HORIZONTAL
            setBackgroundColor(getColor(R.color.wear_black))
        }

        scrollView = ScrollView(this).apply {
            isFillViewport = true
            setBackgroundColor(getColor(R.color.wear_black))
            addView(container)
        }

        setContentView(scrollView)
        render(currentScreen)
    }

    override fun onStart() {
        super.onStart()
        observation = CoroutineScope(Dispatchers.Main).launch {
            dataLayer.observeSnapshot { snapshot ->
                val prevSnapshot = latestSnapshot
                latestSnapshot = snapshot

                if (prevSnapshot?.state != ConnectionState.READY && snapshot.state == ConnectionState.READY) {
                    if (currentScreen is Screen.Status) {
                        screenStack.clear()
                        currentScreen = Screen.Home
                        render(currentScreen)
                    }
                } else if (currentScreen is Screen.Status) {
                    render(Screen.Status(snapshot.state))
                }
            }
        }
    }

    override fun onStop() {
        observation?.cancel()
        observation = null
        dismissJob?.cancel()
        dismissJob = null
        super.onStop()
    }

    @Suppress("DEPRECATION")
    @android.annotation.SuppressLint("GestureBackNavigation")
    override fun onBackPressed() {
        handleBack()
    }

    private fun handleBack() {
        dismissJob?.cancel()
        dismissJob = null
        if (screenStack.isNotEmpty()) {
            currentScreen = screenStack.removeLast()
            render(currentScreen)
        } else {
            if (currentScreen != Screen.Home && latestSnapshot?.state == ConnectionState.READY) {
                currentScreen = Screen.Home
                render(currentScreen)
            } else {
                finish()
            }
        }
    }

    private fun navigateTo(screen: Screen) {
        dismissJob?.cancel()
        dismissJob = null
        screenStack.addLast(currentScreen)
        currentScreen = screen
        render(screen)
    }

    private fun navigateHome() {
        dismissJob?.cancel()
        dismissJob = null
        screenStack.clear()
        currentScreen = Screen.Home
        render(currentScreen)
    }

    private fun activePeople(): List<PersonData> {
        val schedule = latestSnapshot?.schedule
        return if (schedule != null && schedule.people.isNotEmpty()) schedule.people else defaultPeople
    }

    private fun activeMedications(): List<MedicationData> {
        val schedule = latestSnapshot?.schedule
        return if (schedule != null && schedule.medications.isNotEmpty()) schedule.medications else defaultMedications
    }

    private fun render(screen: Screen) {
        container.removeAllViews()
        scrollView.scrollTo(0, 0)
        when (screen) {
            is Screen.Status -> renderStatus(screen.state)
            is Screen.Home -> renderHome()
            is Screen.PersonChooser -> renderPersonChooser()
            is Screen.MedicationChooser -> renderMedicationChooser(screen.person)
            is Screen.TakeNow -> renderTakeNow(screen.person, screen.medication)
            is Screen.DoseRecorded -> renderDoseRecorded(screen.medication, screen.isReachable)
        }
    }

    private fun renderStatus(state: ConnectionState) {
        val messageRes = when (state) {
            ConnectionState.PHONE_APP_MISSING -> R.string.phone_app_missing
            ConnectionState.DISCONNECTED -> R.string.disconnected
            ConnectionState.WAITING_FOR_STATUS -> R.string.waiting_for_status
            ConnectionState.INCOMPATIBLE -> R.string.incompatible
            ConnectionState.SIGNED_OUT -> R.string.signed_out
            ConnectionState.READY -> R.string.ready
        }

        val textView = TextView(this).apply {
            textSize = 16f
            gravity = Gravity.CENTER
            setTextColor(getColor(R.color.wear_text_primary))
            val pad = dp(24)
            setPadding(pad, pad, pad, pad)
            setText(messageRes)
        }
        container.addView(textView)

        val exploreButton = createActionButton("Open Offline", getColor(R.color.wear_cancel_bg)) {
            navigateTo(Screen.Home)
        }
        container.addView(exploreButton)
    }

    private fun renderHome() {
        val titleView = createHeader(getString(R.string.app_name))
        container.addView(titleView)

        val meds = activeMedications()
        val people = activePeople()
        val nextDueMed = meds.firstOrNull { it.status == "due_now" } ?: meds.firstOrNull()

        if (nextDueMed != null) {
            val assignedPerson = people.firstOrNull { it.id == nextDueMed.personId } ?: people.first()
            val nextDoseCard = LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                val pad = dp(12)
                setPadding(pad, pad, pad, pad)
                background = createPillBackground(getColor(R.color.wear_card_bg), getColor(R.color.wear_cyan), dp(2))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    setMargins(0, 0, 0, dp(12))
                }

                val tag = TextView(this@MainActivity).apply {
                    text = getString(R.string.next_dose)
                    textSize = 11f
                    setTextColor(getColor(R.color.wear_cyan))
                    typeface = Typeface.DEFAULT_BOLD
                    gravity = Gravity.CENTER
                }
                addView(tag)

                val medName = TextView(this@MainActivity).apply {
                    text = nextDueMed.name
                    textSize = 16f
                    setTextColor(getColor(R.color.wear_text_primary))
                    typeface = Typeface.DEFAULT_BOLD
                    gravity = Gravity.CENTER
                    setPadding(0, dp(2), 0, 0)
                }
                addView(medName)

                val details = TextView(this@MainActivity).apply {
                    text = "${assignedPerson.name} • ${nextDueMed.dose}"
                    textSize = 12f
                    setTextColor(getColor(R.color.wear_text_secondary))
                    gravity = Gravity.CENTER
                    setPadding(0, dp(2), 0, dp(8))
                }
                addView(details)

                val takeNowBtn = createActionButton(getString(R.string.take_now), getColor(R.color.wear_green)) {
                    navigateTo(Screen.TakeNow(assignedPerson, nextDueMed))
                }
                addView(takeNowBtn)
            }
            container.addView(nextDoseCard)
        }

        val personMenuButton = createActionButton(getString(R.string.select_person), getColor(R.color.wear_card_bg)) {
            navigateTo(Screen.PersonChooser)
        }
        container.addView(personMenuButton)

        val statusText = TextView(this).apply {
            val isReady = latestSnapshot?.state == ConnectionState.READY
            text = if (isReady) "Connected" else "Offline"
            textSize = 11f
            setTextColor(if (isReady) getColor(R.color.wear_green) else getColor(R.color.wear_amber))
            gravity = Gravity.CENTER
            setPadding(0, dp(8), 0, 0)
            isClickable = true
            setOnClickListener {
                navigateTo(Screen.Status(latestSnapshot?.state ?: ConnectionState.DISCONNECTED))
            }
        }
        container.addView(statusText)
    }

    private fun renderPersonChooser() {
        container.addView(createHeader(getString(R.string.select_person)))

        val people = activePeople()
        people.forEach { person ->
            val typeLabel = when (person.type) {
                0 -> "Adult"
                1 -> "Child"
                2 -> "Dependent"
                else -> ""
            }
            val chip = createChip(person.name, typeLabel) {
                navigateTo(Screen.MedicationChooser(person))
            }
            container.addView(chip)
        }

        container.addView(createActionButton(getString(R.string.cancel), getColor(R.color.wear_cancel_bg)) {
            handleBack()
        })
    }

    private fun renderMedicationChooser(person: PersonData) {
        container.addView(createHeader(person.name))
        container.addView(createSubHeader(getString(R.string.medications)))

        val allMeds = activeMedications()
        val personMeds = allMeds.filter { it.personId == person.id }.ifEmpty { allMeds }

        if (personMeds.isEmpty()) {
            val emptyText = TextView(this).apply {
                text = getString(R.string.no_medications)
                textSize = 13f
                setTextColor(getColor(R.color.wear_text_secondary))
                gravity = Gravity.CENTER
                setPadding(0, dp(16), 0, dp(16))
            }
            container.addView(emptyText)
        } else {
            personMeds.forEach { med ->
                val isDue = med.status == "due_now"
                val subtitle = if (isDue) {
                    "${med.dose} • ${getString(R.string.due_now)}"
                } else if (med.status == "as_needed") {
                    "${med.dose} • ${getString(R.string.as_needed)}"
                } else {
                    "${med.dose}${med.dueTime?.let { " • $it" } ?: ""}"
                }

                val chip = createChip(med.name, subtitle, highlight = isDue) {
                    navigateTo(Screen.TakeNow(person, med))
                }
                container.addView(chip)
            }
        }

        container.addView(createActionButton(getString(R.string.cancel), getColor(R.color.wear_cancel_bg)) {
            handleBack()
        })
    }

    private fun renderTakeNow(person: PersonData, med: MedicationData) {
        val title = createHeader(med.name)
        container.addView(title)

        val info = TextView(this).apply {
            text = "${med.dose}\nFor ${person.name}"
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(getColor(R.color.wear_text_secondary))
            setPadding(0, 0, 0, dp(16))
        }
        container.addView(info)

        val takeButton = createActionButton(getString(R.string.take_now), getColor(R.color.wear_green)) {
            val isReachable = latestSnapshot?.isPhoneReachable == true
            val record = TakenDoseRecord(
                eventId = UUID.randomUUID().toString(),
                personId = person.id,
                medicationId = med.id,
                takenAt = System.currentTimeMillis(),
                dosage = med.dose
            )
            CoroutineScope(Dispatchers.Main).launch {
                try {
                    dataLayer.recordDose(record)
                } catch (_: Exception) {}
            }
            navigateTo(Screen.DoseRecorded(med, isReachable))
        }
        container.addView(takeButton)

        val cancelButton = createActionButton(getString(R.string.cancel), getColor(R.color.wear_cancel_bg)) {
            handleBack()
        }
        container.addView(cancelButton)
    }

    private fun renderDoseRecorded(med: MedicationData, isReachable: Boolean) {
        val icon = TextView(this).apply {
            text = "✓"
            textSize = 36f
            gravity = Gravity.CENTER
            val badgeColor = if (isReachable) getColor(R.color.wear_green) else getColor(R.color.wear_amber)
            setTextColor(badgeColor)
            val size = dp(56)
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.argb(40, Color.red(badgeColor), Color.green(badgeColor), Color.blue(badgeColor)))
            }
            layoutParams = LinearLayout.LayoutParams(size, size).apply {
                gravity = Gravity.CENTER
                setMargins(0, dp(8), 0, dp(8))
            }
        }
        container.addView(icon)

        val title = TextView(this).apply {
            text = getString(R.string.dose_recorded)
            textSize = 17f
            gravity = Gravity.CENTER
            setTextColor(getColor(R.color.wear_text_primary))
            typeface = Typeface.DEFAULT_BOLD
            setPadding(0, dp(4), 0, 0)
        }
        container.addView(title)

        val medText = TextView(this).apply {
            text = med.name
            textSize = 13f
            gravity = Gravity.CENTER
            setTextColor(getColor(R.color.wear_text_secondary))
            setPadding(0, dp(2), 0, dp(8))
        }
        container.addView(medText)

        val syncStatus = TextView(this).apply {
            text = if (isReachable) getString(R.string.synced_to_phone) else getString(R.string.saved_offline)
            textSize = 11f
            gravity = Gravity.CENTER
            val color = if (isReachable) getColor(R.color.wear_green) else getColor(R.color.wear_amber)
            setTextColor(color)
            val padH = dp(12)
            val padV = dp(4)
            setPadding(padH, padV, padH, padV)
            background = createPillBackground(Color.argb(35, Color.red(color), Color.green(color), Color.blue(color)))
        }
        container.addView(syncStatus)

        dismissJob = CoroutineScope(Dispatchers.Main).launch {
            delay(2500)
            navigateHome()
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private fun createHeader(title: String): TextView = TextView(this).apply {
        text = title
        textSize = 16f
        gravity = Gravity.CENTER
        setTextColor(getColor(R.color.wear_text_primary))
        typeface = Typeface.DEFAULT_BOLD
        setPadding(0, 0, 0, dp(10))
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        )
    }

    private fun createSubHeader(title: String): TextView = TextView(this).apply {
        text = title
        textSize = 12f
        gravity = Gravity.CENTER
        setTextColor(getColor(R.color.wear_text_secondary))
        setPadding(0, 0, 0, dp(8))
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        )
    }

    private fun createPillBackground(bgColor: Int, strokeColor: Int? = null, strokeWidth: Int = 0): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = dp(20).toFloat()
            setColor(bgColor)
            if (strokeColor != null && strokeWidth > 0) {
                setStroke(strokeWidth, strokeColor)
            }
        }

    private fun createChip(title: String, subtitle: String? = null, highlight: Boolean = false, onClick: () -> Unit): Button =
        Button(this).apply {
            val content = if (subtitle != null) {
                val ssb = SpannableStringBuilder(title).append("\n").append(subtitle)
                ssb.setSpan(
                    AbsoluteSizeSpan(dp(12)),
                    title.length + 1,
                    ssb.length,
                    Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
                )
                ssb.setSpan(
                    ForegroundColorSpan(
                        if (highlight) getColor(R.color.wear_cyan) else getColor(R.color.wear_text_secondary)
                    ),
                    title.length + 1,
                    ssb.length,
                    Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
                )
                ssb
            } else {
                SpannableStringBuilder(title)
            }
            this.text = content
            textSize = 14f
            setTextColor(getColor(R.color.wear_text_primary))
            typeface = Typeface.DEFAULT_BOLD
            transformationMethod = null
            gravity = Gravity.CENTER
            val padH = dp(14)
            val padV = dp(10)
            setPadding(padH, padV, padH, padV)
            val bgColor = getColor(R.color.wear_card_bg)
            val strokeColor = if (highlight) getColor(R.color.wear_cyan) else null
            background = createPillBackground(bgColor, strokeColor, if (highlight) dp(2) else 0)
            minHeight = dp(52)
            setOnClickListener { onClick() }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(0, 0, 0, dp(8))
            }
        }

    private fun createActionButton(text: String, bgColor: Int, onClick: () -> Unit): Button =
        Button(this).apply {
            this.text = text
            textSize = 14f
            setTextColor(getColor(R.color.wear_text_primary))
            typeface = Typeface.DEFAULT_BOLD
            background = createPillBackground(bgColor)
            minHeight = dp(44)
            setOnClickListener { onClick() }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(0, 0, 0, dp(8))
            }
        }
}
