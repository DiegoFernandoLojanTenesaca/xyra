package com.indagalab.xyra

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.net.Inet4Address
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import kotlin.concurrent.thread

class MainActivity : TauriActivity() {
  private companion object {
    const val NOTIFICATIONS_REQUEST = 1
    const val UPDATE_FILE = "xyra-update.apk"
    const val APK_TYPE = "application/vnd.android.package-archive"
    const val DOWNLOAD_TIMEOUT_MS = 30000
    const val CHUNK_BYTES = 64 * 1024
    const val PROGRESS_STEP = 5
  }

  private var content: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge(SystemBarStyle.dark(Color.TRANSPARENT), SystemBarStyle.dark(Color.TRANSPARENT))
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    content = webView
    keepClearOfSystemBars()
    webView.addJavascriptInterface(QrBridge(), "XyraQr")
    webView.addJavascriptInterface(WatchBridge(), "XyraWatch")
    webView.addJavascriptInterface(UpdateBridge(), "XyraUpdate")
    webView.addJavascriptInterface(NetworkBridge(), "XyraNetwork")
  }

  /** Keeps the page out from under the status bar, the navigation buttons and the keyboard. */
  private fun keepClearOfSystemBars() {
    val root = findViewById<View>(android.R.id.content)
    root.setBackgroundColor(ContextCompat.getColor(this, R.color.chrome))
    ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
      view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, keyboard.bottom))
      WindowInsetsCompat.CONSUMED
    }
  }

  /** Google's QR scanner, which needs no camera permission and also takes the code typed in; the page gets the result in
   * window.__xyraQrResult. */
  private inner class QrBridge {
    @JavascriptInterface
    fun scan() {
      runOnUiThread {
        val options = GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).allowManualInput().build()
        GmsBarcodeScanning.getClient(this@MainActivity, options)
          .startScan()
          .addOnSuccessListener { deliver(it.rawValue) }
          .addOnCanceledListener { deliver(null) }
          .addOnFailureListener { deliver(null) }
      }
    }
  }

  /** The wait for matches that goes on with the app closed; the page sends the PC's addresses and the texts to show. */
  private inner class WatchBridge {
    @JavascriptInterface
    fun start(settings: String) {
      askForNotifications()
      MatchWatch.start(this@MainActivity, settings)
    }

    @JavascriptInterface
    fun stop() = MatchWatch.stop(this@MainActivity)

    @JavascriptInterface
    fun test() = MatchWatch.test(this@MainActivity)
  }

  /** Installs a newer Xyra: downloads the APK, checks its SHA-256 and opens Android's installer, which keeps the data. */
  private inner class UpdateBridge {
    @JavascriptInterface
    fun version(): String = BuildConfig.VERSION_NAME

    @JavascriptInterface
    fun install(url: String, sha256: String) {
      thread {
        try {
          val apk = download(url, sha256)
          val uri = FileProvider.getUriForFile(this@MainActivity, "$packageName.fileprovider", apk)
          report("installing", 100)
          startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, APK_TYPE).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        } catch (e: Exception) {
          report("failed", 0)
        }
      }
    }
  }

  /** The phone's Wi-Fi, so the page can tell whether it shares a network with the PC. */
  private inner class NetworkBridge {
    @JavascriptInterface
    fun current(): String {
      val manager = getSystemService(ConnectivityManager::class.java)
      @Suppress("DEPRECATION")
      val wifi = manager.allNetworks.firstOrNull { manager.getNetworkCapabilities(it)?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true }
      val address = wifi?.let { network -> manager.getLinkProperties(network)?.linkAddresses?.map { it.address }?.firstOrNull { it is Inet4Address } }
      return JSONObject().put("address", address?.hostAddress ?: "").put("wifi", wifi != null).toString()
    }
  }

  private fun askForNotifications() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
    if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) return
    runOnUiThread { requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), NOTIFICATIONS_REQUEST) }
  }

  private fun download(url: String, sha256: String): File {
    val apk = File(cacheDir, UPDATE_FILE)
    val connection = URL(url).openConnection() as HttpURLConnection
    connection.connectTimeout = DOWNLOAD_TIMEOUT_MS
    connection.readTimeout = DOWNLOAD_TIMEOUT_MS
    val digest = MessageDigest.getInstance("SHA-256")
    try {
      if (connection.responseCode !in 200..299) throw IOException("HTTP ${connection.responseCode}")
      val total = connection.contentLengthLong
      connection.inputStream.use { input ->
        apk.outputStream().use { output ->
          val buffer = ByteArray(CHUNK_BYTES)
          var received = 0L
          var reported = -1L
          while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            output.write(buffer, 0, read)
            digest.update(buffer, 0, read)
            received += read
            val percent = if (total > 0) 100 * received / total else 0
            if (percent / PROGRESS_STEP != reported / PROGRESS_STEP) {
              reported = percent
              report("progress", percent.toInt())
            }
          }
        }
      }
    } finally {
      connection.disconnect()
    }
    val hash = digest.digest().joinToString("") { "%02x".format(it) }
    if (sha256.isNotEmpty() && hash != sha256.lowercase()) {
      apk.delete()
      throw IOException("checksum")
    }
    return apk
  }

  private fun report(status: String, percent: Int) = call("__xyraUpdate", "${JSONObject.quote(status)}, $percent")

  private fun deliver(raw: String?) = call("__xyraQrResult", if (raw == null) "null" else JSONObject.quote(raw))

  private fun call(callback: String, arguments: String) {
    runOnUiThread { content?.evaluateJavascript("window.$callback && window.$callback($arguments)", null) }
  }
}
