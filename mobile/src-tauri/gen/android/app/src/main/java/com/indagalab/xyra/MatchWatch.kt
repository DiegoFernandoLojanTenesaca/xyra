package com.indagalab.xyra

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.net.Uri
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import kotlin.concurrent.thread

/** Starts and stops the wait for matches; its settings stay saved so it comes back after the phone restarts. */
object MatchWatch {
  private const val PREFERENCES = "match-watch"
  private const val SETTINGS = "settings"

  fun start(context: Context, settings: String) {
    context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).edit().putString(SETTINGS, settings).apply()
    ContextCompat.startForegroundService(context, Intent(context, MatchWatchService::class.java))
  }

  /** Shows the match notification once, so the player can hear it and try its button. */
  fun test(context: Context) {
    context.startService(Intent(context, MatchWatchService::class.java).setAction(MatchWatchService.TEST))
  }

  fun stop(context: Context) {
    context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).edit().remove(SETTINGS).apply()
    context.stopService(Intent(context, MatchWatchService::class.java))
  }

  fun settings(context: Context): JSONObject? =
    context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getString(SETTINGS, null)?.let { JSONObject(it) }
}

/** Waits for matches again once the phone turns on, when the player left it on. */
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action == Intent.ACTION_BOOT_COMPLETED) MatchWatch.settings(context)?.let { MatchWatch.start(context, it.toString()) }
  }
}

/** The PC stopped knowing this phone: it was disconnected from Xyra's settings. */
private class Unpaired : IOException()

/** Follows Xyra on the PC as the app does and rings when a match is found, with a button to accept it. */
class MatchWatchService : Service() {
  companion object {
    const val TEST = "com.indagalab.xyra.TEST"
    private const val WATCHING_CHANNEL = "watching"
    /** A channel's sound cannot change once created, so the one with League's sound has its own name. */
    private const val MATCH_CHANNEL = "match-found"
    private const val OLD_MATCH_CHANNEL = "match"
    private const val WATCHING_ID = 1
    private const val MATCH_ID = 2
    private const val ACCEPT = "com.indagalab.xyra.ACCEPT"
    private const val DECLINE = "com.indagalab.xyra.DECLINE"
    /** Marks the accept button of the test notification, which only answers on the phone. */
    private const val FROM_TEST = "test"
    private const val ACCEPT_REQUEST = 1
    private const val TEST_ACCEPT_REQUEST = 2
    private const val DECLINE_REQUEST = 3
    private const val TEST_DECLINE_REQUEST = 4
    private const val CONNECT_TIMEOUT_MS = 3000
    /** The PC answers a state request within 20 s; this leaves room for a slow network. */
    private const val READ_TIMEOUT_MS = 30000
    private const val RETRY_MS = 15000L
    private const val MATCH_SHOWN_MS = 15000L
    private const val CHAMP_SELECT_SHOWN_MS = 120000L
    private const val FORBIDDEN = 403
    private const val CHAMP_SELECT = "champSelect"
  }

  @Volatile private var settings: JSONObject? = null
  @Volatile private var running = false
  private var worker: Thread? = null

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val saved = MatchWatch.settings(this)
    if (saved == null) {
      stopSelf()
      return START_NOT_STICKY
    }
    settings = saved
    channels(saved.getJSONObject("texts"))
    val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE else 0
    ServiceCompat.startForeground(this, WATCHING_ID, watching(saved.getJSONObject("texts")), type)
    val texts = saved.getJSONObject("texts")
    val test = intent?.getBooleanExtra(FROM_TEST, false) == true
    when (intent?.action) {
      ACCEPT -> if (test) show(notice(texts, "accepted", "testDone", MATCH_SHOWN_MS)) else thread { answer("/api/accept", "accepted", "acceptFailed") }
      DECLINE -> if (test) show(notice(texts, "declined", "testDone", MATCH_SHOWN_MS)) else thread { answer("/api/decline", "declined", "acceptFailed") }
      TEST -> show(matchFound(texts, true, test = true))
    }
    if (worker == null) {
      running = true
      worker = thread { watch() }
    }
    return START_STICKY
  }

  override fun onDestroy() {
    running = false
    worker?.interrupt()
    super.onDestroy()
  }

  /** Long-polls the PC's state through each of its addresses in turn, resting a while when none answers. */
  private fun watch() {
    var version: Long? = null
    var host = 0
    var readyCheck = false
    var canAccept = false
    while (running) {
      val current = settings ?: return
      val hosts = current.getJSONArray("hosts")
      val base = "http://${hosts.getString(host % hosts.length())}:${current.getString("port")}"
      try {
        if (version == null) canAccept = request(base, "/api/pc", current).getJSONObject("permissions").optBoolean("accept")
        val answer = request(base, "/api/state" + (version?.let { "?after=$it" } ?: ""), current)
        version = answer.getLong("version")
        val state = answer.getJSONObject("state")
        val found = state.optBoolean("ready_check")
        val texts = current.getJSONObject("texts")
        when {
          found && !readyCheck -> show(matchFound(texts, canAccept))
          !found && readyCheck && state.optString("phase") == CHAMP_SELECT -> show(notice(texts, "champSelect", "champSelectText", CHAMP_SELECT_SHOWN_MS))
          !found && readyCheck -> manager().cancel(MATCH_ID)
        }
        readyCheck = found
      } catch (e: Unpaired) {
        MatchWatch.stop(this)
        return
      } catch (e: InterruptedException) {
        return
      } catch (e: Exception) {
        version = null
        host++
        try {
          if (host % hosts.length() == 0) Thread.sleep(RETRY_MS)
        } catch (e: InterruptedException) {
          return
        }
      }
    }
  }

  /** Accepts or declines the match on the PC and says how it went. */
  private fun answer(path: String, done: String, failed: String) {
    val current = settings ?: return
    val hosts = current.getJSONArray("hosts")
    val answered = (0 until hosts.length()).any { i ->
      runCatching { request("http://${hosts.getString(i)}:${current.getString("port")}", path, current, "POST").optBoolean("ok") }.getOrDefault(false)
    }
    val texts = current.getJSONObject("texts")
    show(if (answered) notice(texts, done, "champSelectText", CHAMP_SELECT_SHOWN_MS) else notice(texts, failed, "matchText", MATCH_SHOWN_MS))
  }

  private fun request(base: String, path: String, settings: JSONObject, method: String = "GET"): JSONObject {
    val separator = if ('?' in path) '&' else '?'
    val token = URLEncoder.encode(settings.getString("token"), "UTF-8")
    val connection = URL("$base$path${separator}t=$token").openConnection() as HttpURLConnection
    connection.requestMethod = method
    connection.connectTimeout = CONNECT_TIMEOUT_MS
    connection.readTimeout = READ_TIMEOUT_MS
    try {
      if (connection.responseCode == FORBIDDEN) throw Unpaired()
      if (connection.responseCode !in 200..299) throw IOException("HTTP ${connection.responseCode}")
      return JSONObject(connection.inputStream.bufferedReader().use { it.readText() })
    } finally {
      connection.disconnect()
    }
  }

  private fun manager() = getSystemService(NotificationManager::class.java)

  private fun show(notification: Notification) = manager().notify(MATCH_ID, notification)

  private fun channels(texts: JSONObject) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    manager().createNotificationChannel(NotificationChannel(WATCHING_CHANNEL, texts.getString("watchingChannel"), NotificationManager.IMPORTANCE_MIN))
    manager().deleteNotificationChannel(OLD_MATCH_CHANNEL)
    val sound = Uri.parse("android.resource://$packageName/${R.raw.match_found}")
    val attributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION_EVENT).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build()
    val match = NotificationChannel(MATCH_CHANNEL, texts.getString("matchChannel"), NotificationManager.IMPORTANCE_HIGH)
    match.setSound(sound, attributes)
    match.enableVibration(true)
    manager().createNotificationChannel(match)
  }

  private fun openApp(): PendingIntent =
    PendingIntent.getActivity(
      this,
      0,
      Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

  private fun base(channel: String, texts: JSONObject, title: String, text: String) =
    NotificationCompat.Builder(this, channel)
      .setSmallIcon(R.drawable.ic_notification)
      .setColor(ContextCompat.getColor(this, R.color.accent))
      .setContentTitle(texts.getString(title))
      .setContentText(texts.getString(text))
      .setContentIntent(openApp())

  private fun watching(texts: JSONObject) =
    base(WATCHING_CHANNEL, texts, "watching", "watchingText").setOngoing(true).setPriority(NotificationCompat.PRIORITY_MIN).build()

  private fun matchFound(texts: JSONObject, canAccept: Boolean, test: Boolean = false): Notification {
    val builder = base(MATCH_CHANNEL, texts, "matchFound", "matchText")
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setCategory(NotificationCompat.CATEGORY_EVENT)
      .setAutoCancel(true)
      .setTimeoutAfter(MATCH_SHOWN_MS)
    if (canAccept) {
      builder.addAction(0, texts.getString("accept"), action(ACCEPT, if (test) TEST_ACCEPT_REQUEST else ACCEPT_REQUEST, test))
      builder.addAction(0, texts.getString("decline"), action(DECLINE, if (test) TEST_DECLINE_REQUEST else DECLINE_REQUEST, test))
    }
    return builder.build()
  }

  private fun action(name: String, request: Int, test: Boolean): PendingIntent =
    PendingIntent.getService(this, request, Intent(this, MatchWatchService::class.java).setAction(name).putExtra(FROM_TEST, test), PendingIntent.FLAG_IMMUTABLE)

  private fun notice(texts: JSONObject, title: String, text: String, shownMs: Long) =
    base(MATCH_CHANNEL, texts, title, text).setSilent(true).setAutoCancel(true).setTimeoutAfter(shownMs).build()
}
