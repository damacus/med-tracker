package io.damacus.medtracker.wear

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.widget.ScrollView
import android.widget.TextView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

class MainActivity : Activity() {
    private lateinit var statusText: TextView
    private var observation: Job? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        statusText = TextView(this).apply {
            textSize = 18f
            gravity = Gravity.CENTER
            val padding = (32 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding, padding, padding)
            accessibilityLiveRegion = android.view.View.ACCESSIBILITY_LIVE_REGION_POLITE
            setText(R.string.waiting_for_status)
        }
        setContentView(ScrollView(this).apply {
            isFillViewport = true
            addView(statusText)
        })
    }

    override fun onStart() {
        super.onStart()
        observation = CoroutineScope(Dispatchers.Main).launch {
            WearDataLayer(applicationContext).observe { state ->
                statusText.setText(when (state) {
                    ConnectionState.PHONE_APP_MISSING -> R.string.phone_app_missing
                    ConnectionState.DISCONNECTED -> R.string.disconnected
                    ConnectionState.WAITING_FOR_STATUS -> R.string.waiting_for_status
                    ConnectionState.INCOMPATIBLE -> R.string.incompatible
                    ConnectionState.SIGNED_OUT -> R.string.signed_out
                    ConnectionState.READY -> R.string.ready
                })
            }
        }
    }

    override fun onStop() {
        observation?.cancel()
        observation = null
        super.onStop()
    }
}
