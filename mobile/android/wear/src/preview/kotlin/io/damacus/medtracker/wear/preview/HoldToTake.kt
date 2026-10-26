package io.damacus.medtracker.wear.preview

internal data class HoldFrame(val progress: Float, val completedNow: Boolean = false)

internal class HoldToTake {
    private var startedAt: Long? = null
    private var completed = false

    fun start(nowNanos: Long): Boolean {
        if (completed) return false
        startedAt = nowNanos
        return true
    }

    fun advance(nowNanos: Long): HoldFrame {
        val start = startedAt ?: return HoldFrame(0f)
        val progress = ((nowNanos - start).coerceAtLeast(0) / 1_000_000_000f).coerceAtMost(1f)
        val completedNow = progress >= 1f && !completed
        if (completedNow) completed = true
        return HoldFrame(progress, completedNow)
    }

    fun cancel() {
        startedAt = null
    }

    fun confirmAccessible(): Boolean {
        if (completed) return false
        completed = true
        startedAt = null
        return true
    }
}
